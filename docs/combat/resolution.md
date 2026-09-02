# Combat Resolution

How a single attack resolves, end to end. The model is a **ballistic simulation** (Xenonauts / UFO: Enemy Unknown lineage), **not** an abstract to-hit roll: accuracy becomes a *dispersion cone*, shots are *real projectiles* that travel and hit the first thing in their path, and cover is a *physical object*. The resolution math is **deterministic** (injectable seeded RNG — randomized per battle by default; a `battle_seed` / `GDTF_BATTLE_SEED` env pins a replay) and lives in the **render-free authoritative sim** (`gdtf_battle_sim` — combat math, the coarse march, the resolve-coarse entry point), kept separate from the presenter/render layer (`gdtf_battle_presenter`), so it can be unit-tested headlessly.

> The *structure* below is settled (decided via design interview) and the shot pipeline is **built** in the source. Every coefficient and curve — accuracy→cone mapping, weapon numbers, clearance band edges, body-part weights, severity distribution, reaction cap, bleed rate, TU costs — is **TBD (tuning)**.

## Pipeline

```text
aim → dispersion cone → ONE 3D shot vector → coarse march (60×60×8 voxel DDA)
   → first thing the round fails to clear: (ganger | cover | floor/roof slab | ground)
     — or a clean miss off the grid
   → if ganger: hit-location (weighted part roll) → damage (matchup + per-hit formula)
   → wound severity bucket roll (every hit; penetration-gated)
```

## 1. Accuracy is dispersion, not hit%

Aiming centers a **dispersion cone on the target**: the cone's central axis runs from the **3D muzzle point** (shooter's cell center + per-facing forward offset in cell-fractions, z = per-stance muzzle height as a level-fraction — `muzzle_position`) to the **aim point** (target's cell center, z = the *target's* silhouette-top level-fraction × `aim_height_frac` — a standing target pins the torso band; a cover-occupied cell is aimed at the object's own band midpoint, so deliberately shooting a low crate works at range). Resolving a shot is **two independent calculations**:

### (a) Cone size — how wide the spread *can* be

The maximum angular deviation. Driven by:

- the **weapon's base spread**.
- **Stability** — a **continuous score** *derived from the situation*, not a binary state: stance (prone 40 / kneel 25 / stand 10) + automatic **brace** (+30 when the faced cell's cover height suits the stance: prone on LOW+, kneeling on MID+, standing on HIGH, **OR** the weapon carries the `stable` tag) + an emplacement term (no entities yet). The score is **stance + brace + emplacement** — there is **no** weapon-intrinsic *points* term: a weapon's only stability contribution is the boolean `stable` tag, which engages the brace **unconditionally** (a bipod-mounted / braced-by-design weapon, steady regardless of faced cover or ganger height — it bypasses the brace min-height gate). Normalised over 100 and fed through a tuning **curve** → the cone multiplier (steadier → **narrower**). The same score's second curve output damps recoil *climb* (below). (Weapon-points model removed 2026-06-15; replaced by the `stable` tag.)
- **Recoil (kickback)** — each round in a burst adds the weapon's kickback, **widening** the cone for the *next* round; resets at the end of each shot action (the next action starts fresh). Recoil also walks the **muzzle up**: round *i*'s central axis tilts upward by `prior_shots × recoil_climb × recoil_growth` radians before sampling — `recoil_growth` is stability's second curve output, so a braced/prone shooter climbs strictly less. **Stability damps the widening too:** the same `recoil_growth` scales the cone-widening term (`recoil = 1 + prior_shots × kickback × recoil_growth`), so a braced/prone shooter not only climbs strictly less but **widens strictly less** per prior shot — symmetric with the climb. (Resolved 2026-06-15; previously this widening was undamped.)
- **Aim Mode** — aimed **narrows** (×0.6), hip-fired = 1; the tradeoff is TU (×1.5 shot cost). A separate axis from the selector.
- **Fire mode** — the selector term: single ≈ 1, full-auto ≥ 1 (inherently sloppier); the selector is an **authored list of the modes the weapon offers — any subset of {Single, Burst, Full}, in authored order** (a `FireMode(Vec<FireModeSpec>)`), not three fixed ladders. Each mode carries a closed `ModeKind` (`Single` / `Burst` / `Full`) whose `Display` is the human label ("single"/"burst"/"full-auto") — there is no stored name string.

All factors are **multiplicative** (`cone_angle`):

```text
θ_cone = base_spread × stability × aim × firemode × recoil
  stability : cone-mult curve over the 0–100 stability score (steadier < 1)
  aim       : Aim Mode ×0.6   · hip-fired = 1
  firemode  : single ≈ 1      · full-auto ≥ 1
  recoil    : 1 + prior_shots × kickback × recoil_growth   (first shot has 0 prior → ×1; steadier recoil_growth widens less)
```

Multiplicative bracing tightens **proportionally** — a bipod helps a heavy, sloppy weapon far more in absolute degrees than it helps a tack-driver, so setting up the big gun is a real payoff. **Scaling recoil** multiplies the weapon's own spread: a sloppy weapon sprays on auto (you must aim/single-fire it) while a tight weapon stays usable on auto (it keeps your options open).

### (b) In-cone vector — where inside the cone the shot goes

A deviation sampled (seeded RNG) from within the cone, **biased toward the center** by a power-law radius (a **hard edge** — a shot never exceeds `θ_cone`):

```text
θ_shot = θ_cone × rand^p        (rand ∈ [0,1))
φ      = rand × 2π              (uniform direction)
p      = concentration, rising with accuracy = Shooting × weapon.accuracy
```

`p ≈ 1` (low accuracy) scatters evenly out to the cone edge; high `p` clusters near **dead-center** (on the target). The weapon term can exceed 1.0. The resulting `(θ_shot, φ)` is sampled as **ONE 3D unit vector** about the central axis (`sample_cone_vector` — lateral *and* vertical scatter in one draw; cone 0 = the axis exactly) — the shot's true direction, which the **coarse march** flies (§2; the §4 silhouette classification it once fed is retired — see the note there).

**The two levers are independent:** stability / kickback / fire-mode set how *wide* the cone can throw a shot; accuracy sets how *likely* it stays near center. A wide cone with high accuracy still mostly lands on target; a narrow cone with poor accuracy is bounded but evenly scattered.

**Range needs no explicit term.** The cone is angular, so the same spread covers more ground than the silhouette at distance — far targets get harder *for free*. No per-tile accuracy falloff.

**Firing arc — a shot requires the target in front (or the TU to turn into front).** A shooter may fire **directly** only at a target inside its **facing arc**: a tunable cone of `firing_arc` degrees centred on the facing (default **~120°**, i.e. **±60°**), tested as a **continuous angle** between the facing unit vector and the actor→target ground vector — `≤ firing_arc / 2` is in-arc (inclusive edge), **not** 8-way snapping. A target **outside** the arc is fired **only when the shooter can afford BOTH the turn-into-arc AND the shot** (`turn_cost = steps_to(target_facing) × turn_tu`, plus the shot's TU): then it **atomically** turns to face the target (`from_cells(actor, target)` — the full turn, not the partial-turn path of a plain right-click), spends the turn TU, and fires. If it cannot afford turn + fire, the shot is **rejected** — **no TU spent, no facing change, no shot** (USER ruling 2026-06-16: "only shoot if the player has enough TU to both turn to take the shot"). The arc is **global** this slice (one width for every weapon; a per-weapon arc is a future refinement). A co-located target (the shooter's own cell) is degenerate → in-arc, no turn.

**The two on-screen readouts quote different numbers on purpose.** The mode-panel button quotes the fire mode's own cost. The fire-target cell label quotes what the shot will charge from where the shooter stands, including any turn needed to face the target. So the same shot can read 24 TU on the button and 28 TU on the label at the same moment, and both are right — the button has no target to price a turn against. The button does **not** show the turn-included cost (USER ruling 2026-08-11). `battle.sightline` is not a third readout: it answers two booleans and quotes no TU, and it passes the shot cost into `can_engage` because `decide_fire_arc` adds the turn itself.

## 2. The projectile travels — misses are real

A shot is a ray from the 3D muzzle point along its (perturbed) direction, marched as a **true 3-axis voxel DDA** (`march_vector`) over the **60×60×8 coarse grid** in **sim units** (cubic voxels — one sim unit = one cell on X = one cell on Y = one level on Z). Per occupied cell it crosses, the projectile's **continuous z within that level** (sim units) is compared to the occupant's band-top height: **strictly higher sails over; equal-or-lower impacts** — cover, the intended target, or **any other actor in the path — including your own gang** (true friendly fire). (Band edges are level-fractions — see §1 stability/brace and the clearance banding below; the bands are the silhouette / cover-height abstraction, not the projectile.) Z-boundary crossings test the **floor/roof slab** (one slab, both faces): an intact slab stops the round. Top exit = clean sky miss; **bottom exit strikes the ground** (damaged, never destroyed — crater FX later); lateral exit = miss. There is **no target stop** — the round flies past the aim cell into whatever's behind; a "miss" is just a shot whose deviation carried it past everything.

**The only hard exception:** the shooter's own cell never blocks its own shot. The old aim-occlusion exception list (the cover *you* are behind, prone allies, smaller allies) is **retired as special-casing** — clearance is purely by height (the round's continuous z vs the occupant's band-top), which reproduces it for flat fire: a standing shooter's HIGH round clears the LOW crate it braces on and a prone ally by height, not by exemption (a *prone* shooter genuinely cannot clear even LOW cover — intended).

## 3. Cover is a physical object

Cover has a **height** and its **own armor stats + HP** — it uses the same armor/damage model as a ganger ([weapons-and-armor.md](weapons-and-armor.md)). A shot can strike cover instead of the target, **damaging and potentially destroying it**. One object per cell; it occupies its cell at its `cover_height` band (LOW / MID / HIGH) and stops any round not flying strictly higher (§2). Destroying it **replaces the piece with whatever its def's `leaves_behind` names** — another authored def, a sprite with no mechanics, or nothing. Pathing and LOS are then read from the piece standing in the cell: the destroyed piece loses `BlocksPathfinding` and `BlocksVision`, and `project_path_blocking` and `project_vision_blocking` carry that onto the change-driven occupancy grid (maintained in place, never rebuilt per shot). A def that leaves nothing behind frees the cell; a def that leaves a blocking piece keeps it blocked. The HP lives on the **model's cover ledger** (one ledger for walls *and* props, keyed (cell, level)): `apply_cover_hit` spends it, and depletion emits a **piece-destroyed event** carrying (cell, level) and the kind of piece destroyed, read by the presenter for its reactions and by the sim systems that react to a piece going: `replace_destroyed_piece` puts what the def leaves behind in the cell and writes `SlabLeftOpen` where a smashed slab leaves no slab standing, which is what drops whatever stood on it, and a destroyed emplacement sets its gunner down — walls are destructible cover too, on every storey. **In Bevy:** one buffered message covers every kind — `TerrainPieceDestroyed { at: CellLevel, kind: TerrainPieceKind }` (`crates/gdtf_battle_sim/src/terrain/occupancy_sync/components.rs`), where the kind is `Wall`, `Cover`, `Emplacement` or `Slab`.

### 3.1 Slabs (floors / roofs) are destructible too

Floor and roof **slabs** span cells at the z-boundaries between storeys (one slab, both faces — the floor of the upper storey is the roof of the lower). Like cover, a slab has its **own HP and armor stats**, using the **same armor/damage model as a ganger and cover** — the same `resolve_hit` damage formula resolves a slab hit against the slab's own armor, and the remaining HP-loss depletes the slab's HP pool. A round crossing a z-boundary strikes the slab there (§2): an intact slab stops it; a sufficiently-damaged slab is **destroyed**.

A **destroyed slab that leaves no slab standing**:

- **Stops blocking rounds.** An intact slab blocks the z-boundary crossing (§2); a destroyed slab lets rounds **pass through** the hole (the march reads the surface grid and stops only on a `Present` slab).
- **Passes line of sight.** The LOS probe uses the **same march**, so a destroyed slab is transparent to sight too — destroying it **reopens the previously-blocked vertical sightline** (the squad fog rebuilds on the piece-destroyed message whatever kind it names — literally the same message a destroyed cover writes).
- **Does NOT become walkable.** A destroyed slab remains **impassable to movement** — it adds no vertical link and changes no pathfinding (movement is out of scope here; vertical traversal stays exactly as authored). Smashing a floor opens a firing/sightline hole, **not** a hole you can walk or climb through.

A def whose `leaves_behind` names another slab puts an intact slab in the cell instead, so the boundary goes on stopping rounds and sight.

A slab depletes over **multiple hits** — its HP pool is **persistent across strikes** (a later round finishes a damaged slab), and a smashed slab is **replaced by whatever its def's `leaves_behind` names** — another slab def, which stands as the new floor, or a sprite with no mechanics or nothing, either of which leaves the cell with no slab standing. The HP lives on the **model's slab ledger** (keyed (cell, level), mirroring the cover ledger). A slab's HP and armor are **authored on its `TerrainDef`** (`sim_kind: Slab(hp, armor_protection, armor_hardness)`, see [terrain-authoring.md](../authoring/terrain-authoring.md)) and seeded into the ledger at setup by `seed_slab_terrain`. A cell with **no authored slab is never given an entry** and cannot be destroyed. Depletion to zero emits the same **piece-destroyed event** carrying (cell, level), with the kind `Slab`, which `replace_destroyed_piece` reads to put the successor slab on the surface grid, or to write `Absent` there and emit `SlabLeftOpen` when it leaves no slab standing; either way the LOS rebuild follows.

**Falls — a slab smashed out from under you drops you.** A slab cell left with no slab standing is **LOS-transparent and stops blocking rounds** (above) and stays **non-walkable** (it adds no vertical link — you can't step or climb through a hole), *but* it is **not inert underfoot**: any **live ganger standing on that cell falls through it**. A def that leaves another slab behind drops nobody. The successor stands in the cell, no `SlabLeftOpen` is written, and the ganger keeps its level. The slab keyed (cell, N) is the **floor of storey N** (the roof of storey N−1) — so the faller is the ganger whose position is at that **same** (cell, N), i.e. `level == N` (the occupant at `N+1` is standing on the *roof*, a different slab, and does not fall). The faller drops to the **highest supported storey below**: scanning down, it lands on the ground (storey 0 always supports) or the first **intact (`Present`) slab** — falling through any `Absent` level on the way, whether that cell was never floored or its slab was smashed and left nothing behind. It lands as one **involuntary** re-position (the occupancy grid re-maintains itself off the position change, exactly as for a voluntary move).

The fall then deals damage through the **same** §5 damage → §6 wound → injury pipeline a shot takes (armor is honored): the magnitude is **linear in the storeys fallen** — `per_storey_damage × storeys_fallen`, a **combat-tuning** leaf — routed as a **Neutral kinetic** hit (the fall has no attacker, so no opposed roll and no attacker-Luck term; the defender's own Toughness/Luck still apply). A non-graze, non-fatal fall wound rolls a named injury from the **shared** per-category injury pool, sampled through the **fall** per-source weighting table (`(category, context, severity)`): the injury *definitions* are the one shared pool, but the *weights* are source-specific — a fall weights a `twisted_ankle` high where a ranged shot does not — so no injury def is duplicated per source. A ganger **braced on a stair** (a stair lower-endpoint occupant) does **not** fall — the stair supports it. The fall emits a **fall-occurred** signal (who fell, from/to storey, storeys fallen) for the presenter's reaction. **Weight is deferred** — the current fork is weight-free (a heavier ganger falling harder is a later tuning axis).

*(User-ruled 2026-06-22 for the destructible slab; the fall mechanic is /. Both presenter reactions have since landed: `swap_destroyed_slab` swaps the tile to the destroyed-slab graphic, and the fall FX reader plays off the fall-occurred signal.)*

### 3.2 Ground is damaged, never destroyed

A round that exits the **bottom** of the voxel column strikes the **ground** (§2): it **accrues** the round's `weapon_damage` onto that cell's per-cell ground-damage accumulator on the surface grid (monotonic — the total only ever grows), and changes **nothing** else — the ground has no HP/armor and is never destroyed, never made impassable, never alters cover/slab/ganger state. Purely cosmetic bookkeeping. *(User-ruled 2026-06-22; the crater render FX reads the accrued total and lands in a later ticket.)*

## 4. Hit location — weighted part roll

When the march stops a round on a **ganger** (§2), a **weighted roll** picks **where on them** — one of six body parts: **Head · Torso · L-Arm · R-Arm · L-Leg · R-Leg** (`roll_body_part` over the tuning `body_part_weights`; defaults Head 6 / Torso 40 / each Arm 12 / each Leg 15 — head rare, torso the bulk). **No per-part geometry**: the coarse model decides *which* ganger by height clearance and *where* on them by chance. There is no separate "did it stay on the silhouette" test — the march's band clearance is the answer.

> **RETIRED — both prior models.** The **probabilistic exposure roll** below (on-silhouette classification + exposure-area weights) was first superseded by a **collision-driven** model (ray-vs-per-part colliders — the part struck IS the part hit), and that collider model was itself **retired** by the coarse pipeline. The exposure text below is kept only as the prior design rationale.

The in-cone vector (§1b) first **classifies** the shot: if its deviation keeps it **on the target's silhouette**, resolve hit-location here; if it lands **wide**, it is a real projectile that continues past the target (§2) and may strike cover, terrain, or another actor.

For an **on-silhouette** shot, a **further roll** picks what was actually struck — the **cover** or one of **six body parts**: **Head · Torso · L-Arm · R-Arm · L-Leg · R-Leg**.

**Exposure model.** The target is a vertical silhouette (ground 0 → top H); each part owns a vertical band with a base area, and cover of height `c` occludes everything below `c`:

- a part's **exposed area** = base_area × (fraction of its band above `c`); the **L/R** parts split their band's area horizontally.
- **stance** reshapes the bands — standing (full height), kneeling (compressed, legs tucked low), prone (very low/flat) — so cover occludes **legs first**, then arms/torso, head last.

**The location roll** is weighted by exposed area across the six parts, with the **summed occluded area as the cover's weight** — but that cover weight **shrinks as accuracy rises** (a tighter cone threads the gap to the exposed silhouette instead of clipping the wall). So skill decides *both* whether the shot stayed on-silhouette (§1b) *and* whether an on-silhouette shot finds flesh vs cover.

Examples: standing behind low cover → legs occluded, head/torso/arms exposed; kneeling behind low cover → almost everything below the lip → mostly cover, rare head shot.

**Range is not a factor here** — it already acted upstream (the cone-vs-`θ_target` test); folding it in again would double-count.

The struck body part is the **location** input to the Injury table (location × severity — see [wounds-and-roster.md](wounds-and-roster.md)). This is the join between the ballistic model and the wound system.

## 5. Damage

On a hit, the per-hit formula resolves weapon (`damage` / `punch` / `shred`) against armor (`floor` / `protection` / `integrity` / `hardness`), with the 7-type matchup modifying **punch & shred** only. Full model: [weapons-and-armor.md](weapons-and-armor.md) and [matchup.md](matchup.md).

## 6. Wound

**Every hit rolls severity** (`roll_severity`) — the old `damage > Toughness` hard gate is gone. The score is gated by **penetrating damage**, so a weak hit can't reach the severe buckets (no 1-dmg amputations):

```text
severity_score = j × pen_damage − k × Toughness + part_mod + weapon.fatal_bias
                 + I × Luck_shooter + roll(−L × Luck_defender .. R)
   (defender's Luck extends the roll's FLOOR down — a chance to shrug the hit off —
    while the ceiling stays R, so a genuinely bad roll is always still possible;
    variance grows with the defender's Luck. The shooter's Luck still pushes up.)
   < e0 → None       (a graze — HP loss only, no wound)
   < e1 → Minor      (costs 1 Wound)
   < e2 → Major      (2)
   < e3 → Critical   (3)
   ≥ e3 → Fatal      (empties the pool — death in battle)
```

Penetrating damage drives the score, **Toughness mitigates**, the **struck part** pushes it (head +12 / torso +5 / arms −2 / legs 0 — severity is **location-dependent** now, *and* the location's column still picks the entry in the Injury table), and the weapon's **Fatal-bias** stacks the table. **Luck is directional fortune**: the shooter's Luck adds to the score (nastier wounds), while the defender's Luck **extends the roll's floor downward** — a chance to shrug the hit off entirely (it can pull a would-be Major down to Minor or None) — *without* ever capping the worst roll, so a lucky defender usually takes lighter wounds but can still, occasionally, eat a bad one. `j`, `k`, `I`, `L`, `R`, and the bucket edges `e0..e3` are tuning values (combat tuning; the per-part mods currently sit as a placeholder code const). See [stats.md](stats.md) and [wounds-and-roster.md](wounds-and-roster.md).

### 6.1 Handedness & the hand count

A weapon's **handedness** (`OneHanded` / `TwoHanded`, see [weapons-and-armor.md](weapons-and-armor.md)) is checked against the shooter's **hand count** in the shared firing guard (`can_fire`): a `OneHanded` weapon needs ≥ 1 working hand, a `TwoHanded` weapon needs both. The hand count is **derived on read** from the injury ledger — it is *not* a stored stat or a derived-stat slot. An arm injury carrying a **DisableHand** effect disables the hand on the struck side (`LeftArm` → left, `RightArm` → right; a `DisableHand` on a non-arm part is inert). The disabled sides are a **set**, so two same-side hand injuries still disable exactly one hand. Default (no arm injury) = two hands.

A hand-disabling injury **also** carries an always-on **1H aim penalty** — a `Modify(Shooting, −N)` effect in the *same* injury (N authored per-injury in its `.injury.ron`, a tunable). It flows through the normal modifier layer onto the cached `Shooting` stat (re-summed every projection, hot-reload-surviving), which feeds the §1b concentration exponent — so a ruined hand both loses two-handed weapons *and* loosens the cone for whatever the ganger can still fire. The penalty is always-on while the injury persists (the approved default; it is **not** a firing-time-only term). This gates **fire only** — the `Wields` relationship is never touched.

## 7. Melee — opposed Fight

**Not yet built** — designed as follows. Close combat is an **opposed roll**, and the **margin scales the blow as a *multiplier*** (never a flat add):

```text
atk = Fight_attacker × roll        (roll ∈ [1−v, 1+v], variance v = tunable)
def = Fight_defender × roll
connect if atk > def
margin      = atk / def − 1        (relative dominance — scale-independent, unbounded)
damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)
```

A connecting hit's damage is **multiplied** by `damage_mult`, then runs the normal **damage → wound** steps (§5–6) and the injury draw every wound takes. The built strike (`melee::resolve_melee_strike`) routes its blow through the ONE shared wound core (`synthesize_wound` — the same `resolve_hit` → `roll_severity` → `apply_hit` → injury-roll fold the ranged and fall paths run), applying `damage_mult` to the RESOLVED (post-armor) hit; a non-graze, non-fatal melee wound therefore draws a named injury from the melee per-source weighting tables (`weighting/<category>.melee.weighting.ron`), which the melee dispatch bridges to the existing `InjuryInflicted` message. `margin` is **relative** (`atk/def − 1`), so it's scale-independent and **rewards a genuine skill gap** — a ganger who clearly out-fights the target hits much harder. Melee still isn't the auto-pick: the big multiplier requires actually out-skilling the foe, **and** closing to melee range means eating reaction fire on the approach (the balancing risk — not a power cap). `mult_min` may be **below 1** (a glancing connect); `k_margin`, `mult_min`, `mult_max`, and `v` are tuning values. Set `mult_max` generously so real skill gaps are felt before the clamp.

## 8. Reaction fire

**Not yet built** — designed as follows. Reactions are **intrinsic to the TU economy** — there is no separate overwatch action. Unspent TU funds reaction shots: when an enemy acts within a ganger's LOS (and the watcher can afford a shot), a **probabilistic opposed check** decides whether the watcher interrupts:

```text
score        = Reactions × (TU_left / TU_max)
P(interrupt) = score_watcher / (score_watcher + score_mover)
max interrupts this enemy turn = cap(Reactions)        (tunable)
```

The **reaction shot itself is a normal shot** (full dispersion and damage) — the check only gates *whether* it fires; **there is no damage modifier for reacting**. Because it's a probability, **no ganger is ever hard-locked out**: a twitchy watcher usually interrupts, a slow one occasionally does, and a mover who spends more TU becomes easier to interrupt (their `score` falls). Spending your own TU lowers your priority too, so **hoarding TU keeps you dangerous** on the enemy turn. Per-turn interrupts are capped by the watcher's `Reactions`, and each shot costs TU. The `cap()` function and an optional **probability clamp** (`p_min`/`p_max`, e.g. 0.05 / 0.95 so the extremes are never an absolute 0% or 100%) are tuning values.

## 9. Downed & bleed-out

The terminal gates are **live** (`apply_hit`): `Wounds ≤ 0` → **Dead** (trumps Downed — death can skip it), else `HP ≤ 0` → **Downed** (incapacitated, alive). The bleed-out clock is **live too**: once per full round (`tick_bleed`, ticked at the enemy-phase start) every un-stabilized Downed ganger gains a **"Bleeding Out"** stack draining a **flat `bleed_rate` Wounds** (tunable) — the stack count = turns down = total Wounds lost, a clock you can read (a per-tick **ganger-bleeding event** puts it on screen). Two invariants: **bleeds tick at turn start** (that once-per-round enemy-phase `TurnStarted` boundary, resolved before the new turn's acts — never mid-turn from act resolution), and **only a ganger who ENTERED the tick already Downed drains** — a just-downed ganger has been down **zero** rounds and loses **no** Wound that tick (so a tick's own injury-HP bleed can down but never chain Alive→Downed→Dead within one tick). When `Wounds ≤ 0` the ganger is **Dead**, through the same once-only terminal gate. **Execute** is **live** as well (`execute_downed`: an 8-adjacent ALIVE executor pays the flat `execute_tu` and finishes a Downed ganger outright — the HUD's Execute button). **Stabilize** is **live too**: an 8-adjacent ALIVE ally pays the flat `stabilize_tu` (`stabilize_downed` — the HUD's Stabilize button, greyed out on the same `stabilize_tu_cost` the sim charges; the panel's offer scan calls `can_stabilize`) to **remove the Downed ganger's "Bleeding Out" condition** — `tick_bleed` drains only gangers still carrying that condition, so it skips them from the next round on: new stacks halt, Wounds already drained stay drained, and the ganger **remains Downed**; surviving to battle-end leads to **recover / capture** (campaign scope). Full state machine: [wounds-and-roster.md](wounds-and-roster.md). (An outright-**Fatal** injury kills directly, skipping Downed.) **TBD (Bevy):** the ganger-bleeding signal is a Bevy `Event`; the exact type lands with the sim crate.

## What's pure math vs sim

The combat-math layer (`gdtf_battle_sim` — render-free; deterministic, seed-replayable, unit-testable; tuning via a combat-tuning data structure) owns:

- cone size: `cone_angle(base_spread, firemode, prior_shots, kickback, recoil_growth, stability, aim) → θ_cone`
- stability: `stability(stable, stance, brace, emplacement, …) → (cone_mult, recoil_growth)` — two curve reads off one 0–100 score; `stable` is the weapon's tag (engages the brace unconditionally), not a points value
- muzzle & aim axis: `muzzle_position` (cell center + per-facing forward offset in cell-fractions, per-stance muzzle z as a level-fraction) · `target_aim_point` (target silhouette-top level-fraction × `aim_height_frac`; a cover cell's band midpoint) · `climb_aim_dir` (recoil climb tilts the axis up)
- in-cone vector: `sample_cone_vector(aim_dir, θ_cone, p, rng) → ONE 3D unit direction` — `p = concentration_p(Shooting, weapon.accuracy)`, concentration toward dead-center rising with it
- clearance banding: `band_for(z above the crossed cell's floor) → LOW | MID | HIGH` (tunable edges)
- hit-location: `roll_body_part(body_part_weights, rng)` — the §4 weighted roll, no geometry (the old on-target / exposure tests are retired)
- damage resolution: `resolve_hit(weapon, armor, tuning) → HitResult` (+ the matchup lookup)
- wound severity: `roll_severity(pen_dmg, toughness, part_mod, fatal_bias, both Lucks, rng) → Severity`
- bleed-out (§9): **live** — `tick_bleed` (the per-round drain + stack clock), `execute_downed` (the adjacent finisher), and `stabilize_downed` (the adjacent dressing that halts the clock)
- opposed checks (melee §7, reaction §8): **designed, not yet built** — they land here when their systems do

The **march itself** ("first thing the round fails to clear") is also model-side, not the presenter's: `resolve_coarse` takes the change-driven occupancy / surface / cover grids (maintained in place, never rebuilt per shot — the / ruling), derives muzzle → axis → cone sample, marches `march_vector` (with the clearance + floor/roof checks inside), rolls the part, and returns a `ShotOutcome` (kind + the struck model object itself + cell/part/band + muzzle/trajectory). **TU bookkeeping is model-authoritative too** (`spend_tu` / `reset_tu` / `can_spend_tu`), and **a charge never clamps**: `spend_tu` hands back the shortfall instead of draining the pool to 0, so an indivisible act the ganger cannot afford — a stance change, a deliberate shove, a melee strike, a shot — is refused **whole**: it does not happen and no TU is spent. The only divisible charge is the partial turn (§"What's tunable"); a free act — the weapon-tag auto-shove that rides a connecting hit — is never unaffordable and still lands at 0 TU. **And so is the whole firing act**: `fire()` owns the economy (validation via `can_fire`, TU charge, ammo clamp, burst loop, per-round spend), `cone_for` / `stability_for` compose θ_cone off ganger state + *model* cover (the HUD stability readout reads the same method), `resolve_and_apply` folds armor → severity → application into ONE act returning a frozen report (corpse-skip discipline inside), and every draw comes from the **model-owned seeded RNG**, injected once at setup. The **firing-arc + turn-to-fire gate** (§1) is also model-authoritative: the fire dispatch (`dispatch_fire`) tests `target_in_arc(facing, actor, target, firing_arc)` and — for an out-of-arc target — front-loads the full turn (spend `turn_cost` + set facing) before delegating the shot to `fire()`, or **rejects** the request when turn + fire is unaffordable (no spend, no turn, no shot). `can_fire` stays the coarse fire-mode + valid-target + fire-TU + ammo guard (the HUD button's shared predicate); the arc + turn-affordability are the **sim's** authority, not duplicated into `can_fire`. The presenter (`gdtf_battle_presenter`) keeps the **targeting flow**, the **player-only fog gate** ("unseen — hold your fire" is player policy — it never enters the shared act), **FX staging** off the emitted reports, and drawing — plus the **LOS hint**, a **thin caller of `has_los`**: a deterministic, cone-free, center-to-center march probe over the **same occupancy + slab + height-band geometry the sim fires through**, so the reticle's blocked-hint, the enemy AI's engagement gate, and `resolve_coarse` share ONE geometry truth. It gates aiming, never resolves a hit.

## Coefficients live in the combat-tuning data

The equation **forms** in this doc are code; every **coefficient** is a **default in the combat-tuning data**. The combat-math functions take tuning values as parameters — **nothing tunable is hardcoded**, so balancing is a data edit, not a code change. **TBD (Bevy):** the tuning store is a Bevy `Resource` (likely backed by a serde-loaded asset) — the concrete representation lands with the sim crate.

What's tunable: the matchup multipliers (×1.33 / ×0.34), the Aim-Mode cone ×0.6 + TU premium, the stability stance/brace contributions + the per-stance brace min-height gate + both curves + the recoil climb, the muzzle/aim geometry (per-stance muzzle height as a level-fraction, the per-facing forward offset in cell-fractions, the aim-height fraction), the clearance band edges (level-fractions), the body-part weights, the severity edges + scales + Luck coefficients, stance-change TU (**all-or-nothing** — a pool that cannot cover it refuses the change outright: the stance holds and nothing is charged; there is no half-crouch), turn TU (charged **per 45deg step** — a plain turn is PARTIAL: the ganger turns as many whole 45deg steps as its TU pool affords and lands partway when it runs out, paying exactly one turn-TU per afforded step; 0 affordable steps = no turn, no charge; default 1 TU/step, a USER DECISION — but the **turn-to-fire** turn (§1 firing arc) is all-or-nothing, gated with the shot), the **firing arc** (the facing-cone width in degrees a shooter may fire within before it must turn-into-arc; default ~120° = ±60°, **global** across weapons this slice), the bleed-out clock (`bleed_rate` Wounds per Downed round + the `execute_tu` finisher and `stabilize_tu` dressing costs), and the `reload_tu` magazine refill. Weapon-side numbers (base_spread, accuracy, kickback, fatal_bias, magazine_size) live on the weapon's components; the per-mode numbers (cone mult / TU% / shots) are fields on each `FireModeSpec` in the weapon's `FireMode` selector list — all authored per-weapon in its `assets/content/weapons/ranged/*.weapon.ron` file. Still TBD with its unbuilt system: the reaction-cap formula. (One placeholder exception to "nothing tunable is hardcoded": the per-part severity mods, a code const.)

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
- **Stability** — a **continuous score** *derived from the situation*, not a binary state: weapon intrinsic + stance (prone 40 / kneel 25 / stand 10) + automatic **brace** (+30 when the faced cell's cover height suits the stance: prone on LOW+, kneeling on MID+, standing on HIGH) + an emplacement seam (no entities yet). Normalised over 100 and fed through a tuning **curve** → the cone multiplier (steadier → **narrower**). The same score's second curve output damps recoil *climb* (below).
- **Recoil (kickback)** — each round in a burst adds the weapon's kickback, **widening** the cone for the *next* round; resets at the end of each shot action (the next action starts fresh). Recoil also walks the **muzzle up**: round *i*'s central axis tilts upward by `prior_shots × recoil_climb × recoil_growth` radians before sampling — `recoil_growth` is stability's second curve output, so a braced/prone shooter climbs strictly less. **Stability damps the widening too:** the same `recoil_growth` scales the cone-widening term (`recoil = 1 + prior_shots × kickback × recoil_growth`), so a braced/prone shooter not only climbs strictly less but **widens strictly less** per prior shot — symmetric with the climb. (Resolved 2026-06-15; previously this widening was undamped.)
- **Aim Mode** — aimed **narrows** (×0.6), hip-fired = 1; the tradeoff is TU (×1.5 shot cost). A separate axis from the selector.
- **Fire mode** — the selector term: single ≈ 1, full-auto ≥ 1 (inherently sloppier); selector is **single / single+burst / single+burst+full-auto**, authored per weapon (a `FireMode`).

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

## 2. The projectile travels — misses are real

A shot is a ray from the 3D muzzle point along its (perturbed) direction, marched as a **true 3-axis voxel DDA** (`march_vector`) over the **60×60×8 coarse grid** in **sim units** (cubic voxels — one sim unit = one cell on X = one cell on Y = one level on Z). Per occupied cell it crosses, the projectile's **continuous z within that level** (sim units) is compared to the occupant's band-top height: **strictly higher sails over; equal-or-lower impacts** — cover, the intended target, or **any other actor in the path — including your own gang** (true friendly fire). (Band edges are level-fractions — see §1 stability/brace and the clearance banding below; the bands are the silhouette / cover-height abstraction, not the projectile.) Z-boundary crossings test the **floor/roof slab** (one slab, both faces): an intact slab stops the round. Top exit = clean sky miss; **bottom exit strikes the ground** (damaged, never destroyed — crater FX later); lateral exit = miss. There is **no target stop** — the round flies past the aim cell into whatever's behind; a "miss" is just a shot whose deviation carried it past everything.

**The only hard exception:** the shooter's own cell never blocks its own shot. The old aim-occlusion exception list (the cover *you* are behind, prone allies, smaller allies) is **retired as special-casing** — clearance is purely by height (the round's continuous z vs the occupant's band-top), which reproduces it for flat fire: a standing shooter's HIGH round clears the LOW crate it braces on and a prone ally by height, not by exemption (a *prone* shooter genuinely cannot clear even LOW cover — intended).

## 3. Cover is a physical object

Cover has a **height** and its **own armor stats + HP** — it uses the same armor/damage model as a ganger ([weapons-and-armor.md](weapons-and-armor.md)). A shot can strike cover instead of the target, **damaging and potentially destroying it**. One object per cell; it occupies its cell at its `cover_height` band (LOW / MID / HIGH) and stops any round not flying strictly higher (§2). Destroying it **frees the cell** — pathing and LOS rebuild, and the smashed cell stays clear because the destroyed-cover set excludes it from the change-driven occupancy grid (maintained in place, never rebuilt per shot). The HP lives on the **model's cover ledger** (one ledger for walls *and* props, keyed (cell, level)): `apply_cover_hit` spends it, and depletion emits a **cover-destroyed event** carrying (cell, level) for the presenter's reactions — walls are destructible cover too, on every storey. **TBD (Bevy):** the cover-destroyed signal is a Bevy `Event` (or `EntityEvent`); the exact type lands with the sim crate.

## 4. Hit location — weighted part roll

When the march stops a round on a **ganger** (§2), a **weighted roll** picks **where on them** — one of six body parts: **Head · Torso · L-Arm · R-Arm · L-Leg · R-Leg** (`roll_body_part` over the tuning `body_part_weights`; defaults Head 6 / Torso 40 / each Arm 15 / each Leg 12 — head rare, torso the bulk). **No per-part geometry**: the coarse model decides *which* ganger by height clearance and *where* on them by chance. There is no separate "did it stay on the silhouette" test — the march's band clearance is the answer.

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

## 7. Melee — opposed Fight

**Not yet built** — designed as follows. Close combat is an **opposed roll**, and the **margin scales the blow as a *multiplier*** (never a flat add):

```text
atk = Fight_attacker × roll        (roll ∈ [1−v, 1+v], variance v = tunable)
def = Fight_defender × roll
connect if atk > def
margin      = atk / def − 1        (relative dominance — scale-independent, unbounded)
damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)
```

A connecting hit's damage is **multiplied** by `damage_mult`, then runs the normal **damage → wound** steps (§5–6). `margin` is **relative** (`atk/def − 1`), so it's scale-independent and **rewards a genuine skill gap** — a ganger who clearly out-fights the target hits much harder. Melee still isn't the auto-pick: the big multiplier requires actually out-skilling the foe, **and** closing to melee range means eating reaction fire on the approach (the balancing risk — not a power cap). `mult_min` may be **below 1** (a glancing connect); `k_margin`, `mult_min`, `mult_max`, and `v` are tuning values. Set `mult_max` generously so real skill gaps are felt before the clamp.

## 8. Reaction fire

**Not yet built** — designed as follows. Reactions are **intrinsic to the TU economy** — there is no separate overwatch action. Unspent TU funds reaction shots: when an enemy acts within a ganger's LOS (and the watcher can afford a shot), a **probabilistic opposed check** decides whether the watcher interrupts:

```text
score        = Reactions × (TU_left / TU_max)
P(interrupt) = score_watcher / (score_watcher + score_mover)
max interrupts this enemy turn = cap(Reactions)        (tunable)
```

The **reaction shot itself is a normal shot** (full dispersion and damage) — the check only gates *whether* it fires; **there is no damage modifier for reacting**. Because it's a probability, **no ganger is ever hard-locked out**: a twitchy watcher usually interrupts, a slow one occasionally does, and a mover who spends more TU becomes easier to interrupt (their `score` falls). Spending your own TU lowers your priority too, so **hoarding TU keeps you dangerous** on the enemy turn. Per-turn interrupts are capped by the watcher's `Reactions`, and each shot costs TU. The `cap()` function and an optional **probability clamp** (`p_min`/`p_max`, e.g. 0.05 / 0.95 so the extremes are never an absolute 0% or 100%) are tuning values.

## 9. Downed & bleed-out

The terminal gates are **live** (`apply_hit`): `Wounds ≤ 0` → **Dead** (trumps Downed — death can skip it), else `HP ≤ 0` → **Downed** (incapacitated, alive). The bleed-out clock is **live too**: once per full round (`tick_bleed`, ticked at the enemy-phase start) every un-stabilized Downed ganger gains a **"Bleeding Out"** stack draining a **flat `bleed_rate` Wounds** (tunable) — the stack count = turns down = total Wounds lost, a clock you can read (a per-tick **ganger-bleeding event** puts it on screen). When `Wounds ≤ 0` the ganger is **Dead**, through the same once-only terminal gate. **Execute** is **live** as well (`execute_downed`: an 8-adjacent ALIVE executor pays the flat `execute_tu` and finishes a Downed ganger outright — the HUD's Execute button). **Stabilize** is **live too**: an 8-adjacent ALIVE ally pays the flat `stabilize_tu` (`stabilize_downed` — the HUD's Stabilize button, enabled off the model's own `can_stabilize` predicate so button and act share one guard set) to set the ganger's `stabilized` flag — `tick_bleed` skips them from the next round on: new stacks halt, Wounds already drained stay drained, and the ganger **remains Downed**; surviving to battle-end leads to **recover / capture** (campaign scope). Full state machine: [wounds-and-roster.md](wounds-and-roster.md). (An outright-**Fatal** injury kills directly, skipping Downed.) **TBD (Bevy):** the ganger-bleeding signal is a Bevy `Event`; the exact type lands with the sim crate.

## What's pure math vs sim

The combat-math layer (`gdtf_battle_sim` — render-free; deterministic, seed-replayable, unit-testable; tuning via a combat-tuning data structure) owns:

- cone size: `cone_angle(base_spread, firemode, prior_shots, kickback, recoil_growth, stability, aim) → θ_cone`
- stability: `stability(weapon_intrinsic, stance, brace, emplacement, …) → (cone_mult, recoil_growth)` — two curve reads off one 0–100 score
- muzzle & aim axis: `muzzle_position` (cell center + per-facing forward offset in cell-fractions, per-stance muzzle z as a level-fraction) · `target_aim_point` (target silhouette-top level-fraction × `aim_height_frac`; a cover cell's band midpoint) · `climb_aim_dir` (recoil climb tilts the axis up)
- in-cone vector: `sample_cone_vector(aim_dir, θ_cone, p, rng) → ONE 3D unit direction` — `p = concentration_p(Shooting, weapon.accuracy)`, concentration toward dead-center rising with it
- clearance banding: `band_for(z above the crossed cell's floor) → LOW | MID | HIGH` (tunable edges)
- hit-location: `roll_body_part(body_part_weights, rng)` — the §4 weighted roll, no geometry (the old on-target / exposure tests are retired)
- damage resolution: `resolve_hit(weapon, armor, tuning) → HitResult` (+ the matchup lookup)
- wound severity: `roll_severity(pen_dmg, toughness, part_mod, fatal_bias, both Lucks, rng) → Severity`
- bleed-out (§9): **live** — `tick_bleed` (the per-round drain + stack clock), `execute_downed` (the adjacent finisher), and `stabilize_downed` (the adjacent dressing that halts the clock)
- opposed checks (melee §7, reaction §8): **designed, not yet built** — they land here when their systems do

The **march itself** ("first thing the round fails to clear") is also model-side, not the presenter's: `resolve_coarse` takes the change-driven occupancy / surface / cover grids (maintained in place, never rebuilt per shot — the GTW-6 / GTW-12 ruling), derives muzzle → axis → cone sample, marches `march_vector` (with the clearance + floor/roof checks inside), rolls the part, and returns a `ShotOutcome` (kind + the struck model object itself + cell/part/band + muzzle/trajectory). **TU bookkeeping is model-authoritative too** (`spend_tu` / `reset_tu` / `can_spend_tu`). **And so is the whole firing act**: `fire()` owns the economy (validation via `can_fire`, TU charge, ammo clamp, burst loop, per-round spend), `cone_for` / `stability_for` compose θ_cone off ganger state + *model* cover (the HUD stability readout reads the same method), `resolve_and_apply` folds armor → severity → application into ONE act returning a frozen report (corpse-skip discipline inside), and every draw comes from the **model-owned seeded RNG**, injected once at setup. The presenter (`gdtf_battle_presenter`) keeps the **targeting flow**, the **player-only fog gate** ("unseen — hold your fire" is player policy — it never enters the shared act), **FX staging** off the emitted reports, and drawing — plus the **LOS hint**, a **thin caller of `has_los`**: a deterministic, cone-free, center-to-center march probe over the **same occupancy + slab + height-band geometry the sim fires through**, so the reticle's blocked-hint, the enemy AI's engagement gate, and `resolve_coarse` share ONE geometry truth. It gates aiming, never resolves a hit.

## Coefficients live in the combat-tuning data

The equation **forms** in this doc are code; every **coefficient** is a **default in the combat-tuning data**. The combat-math functions take tuning values as parameters — **nothing tunable is hardcoded**, so balancing is a data edit, not a code change. **TBD (Bevy):** the tuning store is a Bevy `Resource` (likely backed by a serde-loaded asset) rather than a Godot `.tres` — the concrete representation lands with the sim crate.

What's tunable: the matchup multipliers (×1.33 / ×0.34), the Aim-Mode cone ×0.6 + TU premium, the stability stance/brace contributions + the per-stance brace min-height gate + both curves + the recoil climb, the muzzle/aim geometry (per-stance muzzle height as a level-fraction, the per-facing forward offset in cell-fractions, the aim-height fraction), the clearance band edges (level-fractions), the body-part weights, the severity edges + scales + Luck coefficients, stance-change TU, the bleed-out clock (`bleed_rate` Wounds per Downed round + the `execute_tu` finisher and `stabilize_tu` dressing costs), and the `reload_tu` magazine refill. Weapon-side numbers (base_spread, accuracy, kickback, fatal_bias, magazine_size; per-mode cone mult / TU% / shots) live on the weapon / fire-mode data. Still TBD with its unbuilt system: the reaction-cap formula. (One placeholder exception to "nothing tunable is hardcoded": the per-part severity mods, a code const.)

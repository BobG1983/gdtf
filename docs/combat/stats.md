# Ganger Stats

Two-layer model: **direct attributes** (raw potential) feed **computed combat stats** (the values actually rolled in resolution). Wounds and use-improvement act on this system; the campaign identity layer (XP → skills) sits alongside it.

> **BUILT (GTW-384).** The two-layer model now EXISTS in the sim: a ganger's situation `.ron` authors the **eight direct attributes** only, and `setup_battle` DERIVES every computed combat stat from them via a `derive_stats(attributes, &GangerStatTuning)` pure function (render-free, unit-tested). The weights / divisors / TU params live in a **hot-reloadable `assets/combat/stat_tuning.ron`** (a SEPARATE file from `combat/tuning.ron`), loaded into a `GangerStatTuning` Bevy `Resource`; editing it live re-derives every spawned ganger's stats (clamping current pools to the new maxes, never resetting). The derivation is the **single source of truth** — computed stats are never authored. Exact shipped coefficients / ranges remain **tunable data** (not pinned).

## Direct attributes (8)

Raw, slowly-changing potential. They improve through **use** — the Xenonauts model: exercising a capability trains the attribute behind it (no manual stat-point spend). (Use-improvement is campaign layer — **not yet built**.)

| Attribute | Represents |
|-----------|-----------|
| **Speed** | quickness — literally how fast they are physically|
| **Aim** | innate marksmanship, a measure of hand eye coordination |
| **Strength** | physical power; melee force; carrying capacity |
| **Toughness** | resistance to taking damage / being wounded / Diseases / poisons |
| **Reflexes** | reaction speed, twitchiness |
| **Cool** | nerves under fire, literally "how good are you at keeping your cool" |
| **Grit** | resilience, ability to keep going in the face of pain or terrible odds |
| **Luck** | directional fortune — a shooter's Luck makes the wounds they deal nastier; a target's Luck extends the low end of a hit's severity roll downward — a chance to shrug it off (the worst case / ceiling is unchanged). Feeds the severity roll only, never the computed stats below |

## Computed combat stats (derived)

Derived from attributes (+ gear, − wounds). These are what the game rolls. Starting derivations:

| Stat | Derived from | Role |
|------|--------------|------|
| **Time Units (TU)** | mostly **Speed**, reduced by **Strength** shortfall when over carrying capacity | per-turn action budget |
| **Shooting** | fn(**Aim, Reflexes, Cool**) | base ranged to-hit |
| **Fight** | fn(**Speed, Strength, Grit, Cool**) | base melee to-hit |
| **Reactions** | fn(**Speed, Reflexes, Cool**) | base likelihood and number of enemy-turn responses (reaction fire, maybe other stuff) |
| **HP** | fn(**Grit, Toughness**, + a little **Cool**) | raw in-battle damage pool |
| **Morale** | fn(**Grit, Cool**) | Psychological equivalent of HP |
| **Wounds** | **HP / `wounds_per_hp`** (tunable, ≈10) | life pool (small, < a dozen) — Dead at 0 |
| **Bottle** | **Morale / `bottle_per_morale`** (tunable, ≈10) | psychological life pool — Bottled at 0 |

The two **life pools** derive from their damage pools (a second derivation level): tougher/steadier gangers get a proportionally larger life pool. The derivations are **weighted attribute sums** (`derive_stats`, GTW-384); the weights and divisors are tuning values (the `GangerStatTuning` Bevy `Resource`, loaded from `assets/combat/stat_tuning.ron` and hot-reloadable; defaults are flat 1.0 weights, Cool at 0.5 into HP, divisors ≈10). Rounding rule: the pools are integers, so each `f32` weighted sum rounds **to-nearest** when it lands in a pool. `Downed` / `Dead` / `Bottled` themselves are **state components owned by the combat/death system** (which reads these pools), not stats.

**Live today — derived + consumed (GTW-384):** TU (per-turn budget), Shooting (derived `fn(Aim, Reflexes, Cool)`, feeds shot concentration `p = Shooting × weapon accuracy`), HP (knock-down pool), Wounds (life pool, `HP / wounds_per_hp`). **Derived-but-dormant** — now COMPUTED on the sheet at setup (GTW-384) but with no consumer yet: **Fight** (`fn(Speed, Strength, Grit, Cool)` — melee, GTW-51), **Reactions** (`fn(Speed, Reflexes, Cool)` — reaction fire, not yet built), **Morale** (`fn(Grit, Cool)`) **/ Bottle** (`Morale / bottle_per_morale` — the bottle/morale campaign layer, GTW-13). The eight direct attributes (Speed / Aim / Strength / Toughness / Reflexes / Cool / Grit / Luck) all EXIST as authored ganger components; **Toughness** and **Luck** additionally feed the §6 severity roll directly (they are the two that were live before GTW-384).

## Action economy — Time Units (UFO: Enemy Unknown)

A **TU pool** per turn, now DERIVED from Speed at setup (`tu_base + Speed·tu_per_speed`, GTW-384 — the `tu_base` / `tu_per_speed` live in `stat_tuning.ron`; the over-encumbrance Strength reduction is **not yet built**). Every action — step, turn in place, snap / aimed / auto shot, kneel, etc. — costs TUs, and **unspent TUs fuel reaction fire** on the enemy turn (gated by Reactions — **reaction fire not yet built**). Live today: moves, shots (aimed pays the GTW-40 premium, base × 1.5) and stance changes (GTW-48) spend TU on the model (GTW-123). Deepest tactical texture; the heaviest to tune. (Worth a decision-log entry via `/log-decision`.)

## Damage model — HP and Wounds

- **HP** is the in-battle raw-damage pool.
- Every hit **rolls wound severity** (`roll_severity` in the combat sim — the old hard `dmg > Toughness` gate is gone): a **penetration-gated score** — penetrating damage drives it, **Toughness mitigates**, the struck part and the weapon's **fatal-bias** push it up, both **Luck** stats shape a one-sided random tail — bucketed by tunable edges into **None / Minor / Major / Critical / Fatal**. Some weapons bias toward severe / Fatal wounds — it's hard to be hit by a chainsword and *not* take serious wound damage.
- A Wound applies **immediately** — tier + struck location, spending the life pool even if the ganger stays standing. The named-condition **Injury table** (location × severity → condition + stat dock) and the campaign record are **not yet built** (the Injury Tables card).
- **Two pools, two terminal states:** **HP ≤ 0 → Downed** (incapacitated, alive); **Wounds ≤ 0 → Dead**. HP is the knock-down pool; **Wounds is the life pool**.
- **Wounds** is spent by injuries **by severity** (Minor / Major / Critical ≈ 1 / 2 / 3; a **Fatal** injury empties it outright). Stacked injuries or one Fatal hit → Wounds ≤ 0 → **dead — even at full HP**. **HP damage never kills directly; it downs.**
- A **Downed** ganger (HP gone, Wounds remaining) is alive but dying — **bleed-out / stabilize / execute are live** (a flat per-round `bleed_rate` Wounds drain; an adjacent ally's `stabilize_tu` dressing that halts it; an adjacent `execute_tu` finisher — GTW-53 / GTW-52); recover / capture at battle-end stay campaign scope. Full model: [wounds-and-roster.md](wounds-and-roster.md).

## How wounds apply — conditions + stat hits

An Injury is a **named condition** ("Lost Eye", "Limp", "Bleeds", "Shaken") that carries its own special rule **and usually docks an attribute**. This is the most generative model and the highest-ROI content in the game — the table is where the stories come from. **Not yet built** — today a Wound carries only tier + location. Full design: [wounds-and-roster.md](wounds-and-roster.md).

## Improvement & the campaign layer

All of this is campaign layer — **designed, not yet built**.

- **Stats improve via use** (Xenonauts model) — using a capability trains its attribute.
- **XP is a separate track** that buys **skills / abilities**, not raw stat increases.
- **TBD (future / maybe):** a computed *injury-load* track (too many accumulated injuries → the ganger dies) and a *morale-load* track (too much accumulated morale damage → the ganger leaves the gang). Noted, not scheduled.

# Injury Authoring Guide

How to create, extend, and maintain the injury table — from a new `.injury.ron` file
to adding a brand-new `InjuryEffect` variant end-to-end. This guide documents the
**current, landed state** of the injury system (GTW-405 / GTW-437 / GTW-438 / GTW-440 /
GTW-443 / GTW-444 / GTW-436 / GTW-550) and is the primary reference for content authors
and engineers extending injury mechanics.

---

## Part 1 — Creating a new injury (content authoring)

### 1a. Where the `.ron` file goes

Injuries live under `assets/content/injuries/<CATEGORY>/`, where `CATEGORY` is one of
four pools that map to the body's broad anatomy. (Injuries are one of the two
deliberately BESPOKE loaders — one folder, two asset types, two resources — so
its folder/extension spellings are the one-owner consts in
`crates/gdtf_content_families/src/injuries/layout.rs`, GTW-634; see
[content-families.md](content-families.md) Parts 3–4.)

| Category folder | Body parts that draw from it | Theme |
|-----------------|------------------------------|-------|
| `head/` | Head | Aim, Cool effects |
| `torso/` | Torso | Toughness, HP effects |
| `arm/` | **both** LeftArm AND RightArm | Strength, Aim, DisableHand |
| `leg/` | **both** LeftLeg AND RightLeg | Speed, MovementCostMul |

**Why shared pools?** Both arms and both legs share one category pool each (GTW-440).
A wound to the right arm and a wound to the left arm both draw from the `arm/` pool.
The exact struck side (`LeftArm` / `RightArm`) is recorded on the `GainedInjury` and
drives side-sensitive effects (e.g. `DisableHand`), but the table lookup collapses to
the category via `BodyPart::injury_category()`.

**Key convention:** the file stem minus the `.injury` infix is the injury's
`InjuryName` key — the string a weighting file references. For example,
`arm/shattered_hand.injury.ron` → key `"shattered_hand"`. The key must be
unique across ALL four category folders (the registry is flat).

**`category:` field vs. subfolder:** the `category:` authored in the file EQUALS the
subfolder name by construction — `arm/` → `Arm`, `leg/` → `Leg`, `head/` → `Head`,
`torso/` → `Torso`. A mismatch between the def's category and its subfolder produces
a `warn!` at load time but does not fail the load. Authors set the category directly
(e.g. `category: Leg`) — there is no per-side routing (`LeftLeg` / `RightLeg`) in
the authored field.

### 1b. The `.injury.ron` schema — every field with per-line comments

Follow the per-line-comment convention (`.ron-files-commented` project rule):

```ron
// Example: a leg injury ("Hampered" — moves slower, Speed debuffed)
// KEY = file stem minus `.injury` => `twisted_ankle`
(
    name:         "Twisted Ankle",                              // display InjuryName (inspect-panel label)
    category:     Leg,                                          // routes into the Leg category pool; matches the owning subfolder
    severity:     Minor,                                        // Minor | Major | Critical  (None/Fatal are never tabled)
    popup_text:   "ANKLE TWISTED",                             // floating combat text (FCT) shown on infliction
    log_text:     "twists an ankle",                           // combat-log clause ("<name> twists an ankle")
    inspect_text: "Twisted Ankle -- movement slightly impaired",  // persistent inspect-panel description
    effects: [
        MovementCostMul(1.25),                                  // each step costs 1.25x TU (>= 1.0 = slower)
        Modify(stat: Speed, amount: -1),                        // docks the Speed attribute (-1)
    ],
    // post_heal omitted -> defaults to Deferred (GTW-23 Healing owns the semantics)
)
```

**Field reference:**

| Field | Type | Notes |
|-------|------|-------|
| `name` | `InjuryName` (quoted string) | Display label; side-agnostic for shared-pool injuries |
| `category` | `InjuryCategory` enum | Authoritative pool routing — must match the owning subfolder: `Head`, `Torso`, `Arm`, `Leg` |
| `severity` | `Severity` enum | `Minor` / `Major` / `Critical` only — `None` and `Fatal` are never authored |
| `popup_text` | `PopupText` (quoted string) | FCT line shown on infliction |
| `log_text` | `LogText` (quoted string) | Combat-log clause |
| `inspect_text` | `InspectText` (quoted string) | Persistent inspect-panel description |
| `effects` | `Vec<InjuryEffect>` (≥ 1) | The mechanical effects — see the effect vocabulary below |
| `post_heal` | `PostHeal` enum | **Omit** — defaults to `Deferred` (GTW-23 not yet built; see note below) |

**Severity semantics:**

| Tier | Wounds cost | Battle effect |
|------|------------|---------------|
| `Minor` | 1 | Light, stays in fight |
| `Major` | 2 | Significant; benched until healed (campaign) |
| `Critical` | 3 | Severe; benched + healing risk (campaign) |

**`post_heal` note (GTW-23):** omit `post_heal:` entirely — the field defaults to
`Deferred`, which is the only live variant. When GTW-23 Healing lands it will add
`Clean` and `Partial` variants (a clean or partial heal with optional residual effects);
until then the authored value is parsed but never read at runtime. Omitting the field is
both correct and forward-compatible.

### 1c. The weighting file — adding the injury to a bucket

Weighting files live in `assets/content/injuries/weighting/`, **one per
`(category, context)`** (GTW-452): a category has one file per wound SOURCE — the
ranged shot, the melee strike, and the fall — and all of them weight the **same shared
per-category injury pool**. No injury definition is ever duplicated per source; only the
weights differ.

| Source (`context:`) | File name | Live producer today |
|---------------------|-----------|---------------------|
| `Ranged` | `<category>.weighting.ron` | the ranged fire fold (`resolve_and_apply`) |
| `Melee` | `<category>.melee.weighting.ron` | **none yet** — see the dormant-table note below |
| `Fall` | `<category>.fall.weighting.ron` | the fall damage path (`falls::resolve_fall_hit`) |

To make a new injury rollable from a given source, add it to the appropriate bucket in
that source's file:

```ron
// assets/content/injuries/weighting/leg.weighting.ron (abbreviated)
(
    category: Leg,                 // routes into the Leg category pool; matches the owning subfolder
    context:  Ranged,              // the wound SOURCE this table weights (Ranged / Melee / Fall)
    minor: [
        (injury: "twisted_ankle",  weight: 8),   // ADD your new injury here
        // ... other minor entries
    ],
    major: [ /* ... */ ],
    critical: [ /* ... */ ],
)
```

```ron
// assets/content/injuries/weighting/leg.fall.weighting.ron (abbreviated) — the SAME
// shared leg defs, weighted for a fall: landing turns an ankle first.
(
    category: Leg,
    context:  Fall,
    minor: [ (injury: "twisted_ankle", weight: 14) ],
    major: [ /* ... */ ],
    critical: [ /* ... */ ],
)
```

**Weighting rules:**

- `injury:` is the file-stem key (no `.injury.ron` suffix, no path prefix).
- `weight:` is a relative unsigned integer. The roll is a cumulative-weight pick
  over the sorted bucket — higher weight = more likely. The authored order does NOT
  affect determinism (rows are canonically sorted at build time).
- `context:` names the wound source (`Ranged` / `Melee` / `Fall`). It **defaults to
  `Ranged`** when the field is omitted (back-compat with pre-GTW-452 files) — so a
  MISSPELLED `context:` key is silently read as a ranged table and replaces the real
  ranged one. Spell it exactly, and keep the file name and the `context:` field in step.
- One file per `(category, context)`: a second file with the same pair replaces the first
  at build time.
- An unknown key `warn!`s at load and is skipped — never a crash.
- An injury referenced by no bucket in **any** context `warn!`s (it can never be
  rolled) — also never a crash.

**Dormant tables — the melee context (as of GTW-452):** the four
`*.melee.weighting.ron` files are authored, loaded, and built into the tables, but
**nothing samples them yet**: the melee strike path (`melee::resolve_melee_strike`)
resolves damage and a wound severity and stops there — it never runs the §8 injury roll,
so no code constructs `DamageContext::Melee`. The melee tables are therefore live CONTENT
waiting on the melee §8 wiring (a separate ticket); editing them changes nothing in play
until that lands. Ranged and fall tables are both sampled in play today.

**How the struck part maps to the category pool (roll lookup boundary):**
`BodyPart::injury_category()` in `crates/gdtf_battle_sim/src/equipment/armor/stats.rs`
maps the per-side struck part (recorded on the `GainedInjury`) to the shared category
bucket used for the table lookup:

```
LeftArm  / RightArm  -> InjuryCategory::Arm
LeftLeg  / RightLeg  -> InjuryCategory::Leg
Head                 -> InjuryCategory::Head
Torso                -> InjuryCategory::Torso
```

This is the struck-part → category mapping used at roll time, not the authored field.
Both arms / both legs draw from the same built `(Arm, severity)` / `(Leg, severity)`
bucket. The weighting file authors a `category: (Head/Torso/Arm/Leg)` directly — no
per-side `BodyPart` is needed in the authored schema.

---

## Part 2 — Adding a new `InjuryEffect` end-to-end

This section is for engineers adding a new *kind* of mechanical effect. Injury-effect
behaviour lives in the **injury-effect palette** at
`crates/gdtf_battle_sim/src/effects/injuries/` (GTW-550): one self-contained file per
effect implementing the shared `ApplyInjuryEffect` trait (gain-time fold / read-side
projection / heal), plus the closed serde enum in
`crates/gdtf_battle_sim/src/effects/injuries/effect.rs` whose trait impl is a single
mechanical delegation match. STORAGE stays on the ledger
(`crates/gdtf_battle_sim/src/damage_resolution/injuries/ledger.rs`) — the single
`Changed<InflictedInjuries>` source the projector filters on — lent to each effect as
the borrowed `LedgerAccumulators` fold surface. Never give an effect its own
accumulator component.

For an effect that folds into the **existing** accumulators (a summed stat delta, a
summed bleed accrual, the multiplicative movement factor) or is **read-projected**
(like `DisableHand`), the whole job is three steps:

### Step 1 — one palette file

Create `crates/gdtf_battle_sim/src/effects/injuries/<your_effect>.rs` holding:

- the payload newtype, if it has one (no-bare-types rule; `#[serde(transparent)]` so
  it authors as a bare RON scalar),
- the isolated `ApplyYourEffect` behaviour type,
- its `impl ApplyInjuryEffect` — `fold_on_gain` (which accumulator moves, and how:
  summed / multiplicative / documented no-op for a read-projected effect) and `heal`
  (the exact inverse fold, a documented no-op `Ok` when nothing was accumulated, or
  `Err(HealError::NeedsRefold)` when the fold is non-invertible — never a
  `todo!`/`unimplemented!`), plus a projection override (like `disables_hand`) if the
  effect is read-projected,
- a `#[cfg(test)]` unit test asserting the fold/heal semantics.

If the payload contains an `f32`, the enum cannot derive `Eq` — and the drop
**cascades**: every type that transitively carries `InjuryEffect` must also drop `Eq`
(keep `PartialEq`) — `GainedInjury`/`RolledInjury`, `InjuryDef`, `InjuryRegistry`,
`InflictedInjuries`, `InjuryInflicted`, `HitReport`, and the presenter types wrapping
`HitReport`. The compiler surfaces them one crate at a time; note the reason on each
derive's doc-comment. This is safe because the ledger is read via `Changed<>` queries
(tick-based, not `Eq`-based) and none of these types key a `HashSet`/`BTreeMap`. See
`MovementCostMul` (Example B below) for the landed precedent.

### Step 2 — one variant + one delegation arm + one mod line

- `crates/gdtf_battle_sim/src/effects/injuries/effect.rs`: add the `InjuryEffect`
  variant (document its authored RON form: fieldless variants author as
  `VariantName`, tuple variants as `VariantName(payload)`, struct variants as
  `VariantName(field: value, ...)`), and add its one-line arm to the
  `with_behaviour` delegation match — the match is exhaustive, so the compiler
  forces the arm (never a silent no-op).
- `crates/gdtf_battle_sim/src/effects/injuries/mod.rs`: add the `mod <your_effect>;`
  line (and its `pub use` re-export on the same wiring pass).

### Step 3 — author the content

Reference the new variant from an `.injury.ron` `effects:` list and weight the injury
into a bucket (Part 1). Add a ledger-level folding test in
`crates/gdtf_battle_sim/src/damage_resolution/injuries/test/ledger.rs`, and — if the effect
flows through the projector — a projection test in
`crates/gdtf_battle_sim/src/combatants/ganger/test/injury.rs`.

### When it is honestly MORE than three steps

The palette isolates the effect's *own* behaviour; it does not (and cannot) absorb
the READ side, which stays distributed by design. Budget for these when they apply:

- **A genuinely new accumulator kind** (a mutation the existing summed-delta /
  bleed-accrual / movement-product surface cannot express): add the private field +
  public accessor to `InflictedInjuries` in
  `crates/gdtf_battle_sim/src/damage_resolution/injuries/ledger.rs` and surface it as
  one new field on the `LedgerAccumulators` view in
  `crates/gdtf_battle_sim/src/effects/injuries/apply_effect.rs`. Storage stays on the
  ledger — the single `Changed<>` source — never on an effect-owned component.
- **A new `StatTarget`** (a stat that does not exist yet): extend `ALL`, `COUNT`,
  `index()` and `kind()` in
  `crates/gdtf_battle_sim/src/damage_resolution/injuries/stat_target.rs`.
- **A new read-side consumer.** The existing consumers stay where they are under any
  palette shape: the projector
  (`crates/gdtf_battle_sim/src/combatants/ganger/rederive.rs` calling
  `derive_stats_with_injuries` in
  `crates/gdtf_battle_sim/src/combatants/ganger/injury_projection.rs`), the
  pathfinder cost-scale + committed-walk charge (both read
  `movement_cost_factor()`), the `can_fire` hand-count clause (reads
  `hands_available()`), and the bleed runtime (`tick_bleed` queries the
  `BleedAfflicted` mirror). An effect that needs a genuinely NEW runtime — e.g. a
  `Bleeding`-like per-turn drain subtree — will ALWAYS touch that runtime; no
  authoring shape can collapse that, and this guide does not pretend otherwise.

---

## Part 3 — Worked examples

### Example A — `DisableHand` (GTW-443)

`DisableHand` is a **fieldless, inert-at-gain** effect. It disables the hand on the
injury's struck arm.

**RON form:** `DisableHand` (no payload — the disabled side is derived from the
struck `BodyPart` on the `GainedInjury`, not from the effect itself).

**Why fieldless?** The side is already recorded on the `GainedInjury::part` field.
Storing it twice would create a consistency hazard. Two same-side `DisableHand`
injuries must still disable exactly ONE hand — a stored counter could not give that
without de-duping; a set over distinct arm-sides can.

**Fold (`crates/gdtf_battle_sim/src/effects/injuries/disable_hand.rs`):**
`ApplyDisableHand::fold_on_gain` is a documented no-op — nothing accumulated at gain
time. The behaviour is the `disables_hand()` read-side projection the ledger's
`hands_available()` asks of each effect.

**Accessor (`InflictedInjuries::hands_available()`):** folds `self.gained` on demand,
collecting a boolean per arm-side:

```
left_disabled  = any gained entry with DisableHand where part == LeftArm
right_disabled = any gained entry with DisableHand where part == RightArm
HandsAvailable::new(2 - left_disabled as u8 - right_disabled as u8)
```

Parts other than `LeftArm` / `RightArm` are inert (a `DisableHand` on a Torso or Leg
injury has no hand to disable).

**`can_fire` gate:** a `TwoHanded` weapon requires `hands_available() >= 2`. A ganger
with one arm disabled cannot fire a two-handed weapon.

**The 1H aim penalty** a ruined hand also causes rides as a SEPARATE
`Modify(Shooting, -N)` effect in the same injury's `effects` Vec — so the penalty
flows through the normal modifier layer while `DisableHand` only gates two-handed fire.

**Side-from-part (GTW-440 C3):** the def at `arm/shattered_hand.injury.ron` is
side-agnostic (`category: Arm` routes it into the Arm pool). When a wound rolls
`RightArm`, `RolledInjury` is stamped with `part: RightArm` — so the `DisableHand`
disables the RIGHT hand, even though the def was authored with `category: Arm`. The
def's `category` only routes the pool; the struck side (a per-side `BodyPart` on the
gained injury) is authoritative.

---

### Example B — `MovementCostMul` (GTW-444)

`MovementCostMul` is a **multiplicative, accumulated** effect (the "Hampered" status).
It slows movement by multiplying the terrain per-step floor TU cost.

**RON form:** `MovementCostMul(1.5)` — a bare `f32` in parentheses, parsed via
`#[serde(transparent)]` on `MovementCostFactor`.

**Payload newtype:** `MovementCostFactor(f32)` in
`crates/gdtf_battle_sim/src/effects/injuries/movement_cost_mul.rs`. `>= 1.0` means
slower; `1.0` is the identity (no slowdown). CANNOT derive `Eq` (inner is `f32`).

**Fold (`crates/gdtf_battle_sim/src/effects/injuries/movement_cost_mul.rs`):**
`ApplyMovementCostMul::fold_on_gain` runs
`*accumulators.movement = accumulators.movement.times(factor);` — multiplicative,
into the ledger's dedicated accumulator.

**Accumulator field:** `movement: MovementCostFactor` on `InflictedInjuries`,
defaulting to `MovementCostFactor::IDENTITY` (`1.0`). Multiplication is commutative
and associative, so the fold is order-independent and deterministic.

**Stacking rule (LOCKED):** two `MovementCostMul` factors MULTIPLY (`1.5 × 2.0 = 3.0`),
they do NOT add. This is the locked default — do not change without a design ruling.

**Accessor (`InflictedInjuries::movement_cost_factor()`):** returns `self.movement` (the
accumulated product).

**Pathfinder and walk integration:** BOTH the pathfinder cost-function (move-range /
path preview) AND the committed walk's per-step TU charge call
`movement_cost_factor()` and scale each step by it — so the previewed path cost equals
the TU actually charged (GTW-444 C3, preview == charge consistency).

**Part-agnostic:** `MovementCostMul` slows the ganger regardless of which body part
the injury struck. A Leg injury is the natural author, but the fold does not key on
the part.

---

## Part 4 — Hot-reload

The injury system supports **live hot-reload** (GTW-374 pattern): editing any
`.injury.ron` or `.weighting.ron` file while the game is running triggers
`redrive_injuries_on_asset_event` in
`crates/gdtf_app/src/states/load/systems/resolve/injuries.rs`, which rebuilds BOTH
the `InjuryRegistry` and the `InjuryTables` from the persistent
`ActiveInjuriesFolderHandle`.

Because the modifier-layer re-derives on every `Changed<InflictedInjuries>` and on
every `GangerStatTuning` change, a content edit that changes an injury's effects will
be re-applied by the projector on the next derivation — no restart needed.

---

## Part 5 — `post_heal` (GTW-23 — schema-only, not yet built)

Every `.injury.ron` schema carries a `post_heal:` field. **Omit it** — it defaults to
`Deferred`, the only live variant. The field is parsed and stored but never read at
runtime in the current sim.

When GTW-23 Healing lands, `PostHeal` will gain `Clean(..)` and `Partial(..)` variants,
and the runtime will begin reading this field to apply post-battle healing semantics:

- `Clean` — a clean heal that fully clears the injury (often no residual).
- `Partial` — a partial heal that leaves a permanent residual (usually a lasting stat
  debuff).

Until GTW-23 is built, `Deferred` is the correct and forward-compatible choice for all
authored injuries.

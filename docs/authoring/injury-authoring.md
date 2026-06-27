# Injury Authoring Guide

How to create, extend, and maintain the injury table — from a new `.injury.ron` file
to adding a brand-new `InjuryEffect` variant end-to-end. This guide documents the
**current, landed state** of the injury system (GTW-405 / GTW-437 / GTW-438 / GTW-440 /
GTW-443 / GTW-444 / GTW-436) and is the primary reference for content authors and
engineers extending injury mechanics.

---

## Part 1 — Creating a new injury (content authoring)

### 1a. Where the `.ron` file goes

Injuries live under `assets/content/injuries/<CATEGORY>/`, where `CATEGORY` is one of
four pools that map to the body's broad anatomy:

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

**`body_part:` field vs. subfolder:** the `body_part:` authored in the file is
AUTHORITATIVE — the subfolder is only organizational. A mismatch between the def's
category and the subfolder produces a `warn!` at load time but does not fail the load.
Use a `LeftArm` / `RightArm` part to route into the `arm/` pool, and a `LeftLeg` /
`RightLeg` part to route into the `leg/` pool (the loader maps each to its category
via `injury_category()`).

### 1b. The `.injury.ron` schema — every field with per-line comments

Follow the per-line-comment convention (`.ron-files-commented` project rule):

```ron
// Example: a leg injury ("Hampered" — moves slower, Speed debuffed)
// KEY = file stem minus `.injury` => `twisted_ankle`
(
    name:         "Twisted Ankle",                              // display InjuryName (inspect-panel label)
    body_part:    LeftLeg,                                      // routes into the shared Leg pool (LeftLeg or RightLeg -> Leg category)
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
| `body_part` | `BodyPart` enum | Authoritative pool routing: `Head`, `Torso`, `LeftArm`, `RightArm`, `LeftLeg`, `RightLeg` |
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

Each category has one weighting file at
`assets/content/injuries/weighting/<category>.weighting.ron`. To make a new injury
rollable, add it to the appropriate bucket in the matching file:

```ron
// assets/content/injuries/weighting/leg.weighting.ron (abbreviated)
(
    body_part: LeftLeg,            // routes into the Leg category pool
    minor: [
        (injury: "twisted_ankle",  weight: 8),   // ADD your new injury here
        // ... other minor entries
    ],
    major: [ /* ... */ ],
    critical: [ /* ... */ ],
)
```

**Weighting rules:**

- `injury:` is the file-stem key (no `.injury.ron` suffix, no path prefix).
- `weight:` is a relative unsigned integer. The roll is a cumulative-weight pick
  over the sorted bucket — higher weight = more likely. The authored order does NOT
  affect determinism (rows are canonically sorted at build time).
- An unknown key `warn!`s at load and is skipped — never a crash.
- An injury registered but unreferenced by any bucket `warn!`s (it can never be
  rolled) — also never a crash.

**How a per-side part maps to the category pool:**
`BodyPart::injury_category()` in `crates/gdtf_battle_sim/src/equipment/armor/stats.rs`:

```
LeftArm  / RightArm  -> InjuryCategory::Arm
LeftLeg  / RightLeg  -> InjuryCategory::Leg
Head                 -> InjuryCategory::Head
Torso                -> InjuryCategory::Torso
```

The weighting file authors a `body_part:` (`LeftArm` or `LeftLeg` conventionally) that
the loader maps to its category. Both arms / both legs then draw from the same built
`(Arm, severity)` / `(Leg, severity)` bucket.

---

## Part 2 — Adding a new `InjuryEffect` variant end-to-end

This section is for engineers adding a new *kind* of mechanical effect. The effect
vocabulary is an exhaustive match — adding a variant is compile-checked at every use
site. Follow these steps exactly.

### Step 1 — Extend `InjuryEffect` in `effect.rs`

File: `crates/gdtf_battle_sim/src/damage_resolution/injuries/effect.rs`

Add a new variant to `InjuryEffect`. If the payload is a domain value (e.g. a
duration, a percentage), wrap it in a newtype first (no-bare-types rule):

```rust
/// Your new effect — one sentence describing what it does.
///
/// Mechanics: describe how the accumulator works (summed / multiplicative / inert).
/// `post_heal` semantics: if the effect is reversible on heal, note it here (GTW-23).
YourEffect(YourPayloadNewtype),
```

Derive the minimum required: `Clone, Copy, PartialEq, Debug, Deserialize`. If the
payload contains an `f32`, the enum CANNOT derive `Eq` — update the derives on the
affected types (see `MovementCostMul` and the `injury-effect-f32-eq-cascade` memory
entry for the cascade rules).

If you need a new `StatTarget` variant (a stat that does not exist yet), add it to
`StatTarget` in `crates/gdtf_battle_sim/src/damage_resolution/injuries/stat_target.rs`,
extending `ALL`, `COUNT`, `index()`, and `kind()`.

### Step 2 — Add the gain arm in `ledger.rs`

File: `crates/gdtf_battle_sim/src/damage_resolution/injuries/ledger.rs`

The `InflictedInjuries::gain` method has an exhaustive match over `InjuryEffect`. The
compiler will now error on the missing arm — add it. Decide the accumulation strategy:

| Strategy | When to use | Implementation |
|----------|-------------|----------------|
| **Summed delta** | A signed stat shift (like `Modify`) | Add a `StatDeltaLedger::add_delta` call |
| **Summed accrual** | A running total (like `Bleeding`) | Widen to `u16`/`i16` and `saturating_add` |
| **Multiplicative** | A factor (like `MovementCostMul`) | Add a dedicated `f32` field; call `.times()` |
| **Inert at gain** | Set-over-parts (like `DisableHand`) | `=> {}` — fold on read, not on gain |

If you add a dedicated field to `InflictedInjuries`, add a public accessor for it (no
bare field access from outside the module).

### Step 3 — Add the effect-application logic / accessor

For **inert-at-gain** effects (like `DisableHand`): add a read method that folds
`self.gained` on demand — the single-source-of-truth pattern. See
`InflictedInjuries::hands_available()` as the canonical example.

For **accumulated** effects: the accessor is just the field reader (e.g.
`movement_cost_factor()` returns `self.movement`).

For effects that need a runtime trigger (e.g. a per-turn drain like `Bleeding`):
wire the runtime in the appropriate system in the sim. The `BleedAfflicted` component
mirrors the ledger's `bleed` field so the bleed runtime can query it without
carrying the whole ledger.

### Step 4 — Wire into the projector (if it modifies a stat)

If your effect changes a derived stat, the re-derive path must apply it. The projector
lives in `crates/gdtf_battle_sim/src/combatants/ganger/rederive.rs` and calls
`derive_stats_with_injuries` in `combatants/ganger/injury_projection.rs`. If the new
effect docks a stat MAX or shifts a derived value, add the fold there.

### Step 5 — Update `can_fire` and other gates (if it gates an action)

If the effect gates an action (like `DisableHand` gating two-handed fire), add the
gate in the relevant system. For `DisableHand`, the gate lives in the fire-eligibility
check (`can_fire`) which calls `InflictedInjuries::hands_available()`.

### Step 6 — Author the RON variant name (serde)

The RON authoring form is the enum variant's serde name. Fieldless variants author as
`VariantName`; tuple variants as `VariantName(payload)`; struct variants as
`VariantName(field: value, ...)`. Document the authored form in the doc comment.

### Step 7 — Add tests

Add at minimum:

- A unit test in `crates/gdtf_battle_sim/src/damage_resolution/injuries/test.rs` that
  gains an injury carrying your new effect and asserts the accessor returns the expected
  accumulated value.
- If the effect flows through the projector, add a projection test in
  `crates/gdtf_battle_sim/src/combatants/ganger/test/injury.rs`.

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

**Gain arm (`ledger.rs`):** `InjuryEffect::DisableHand => {}` — nothing accumulated at
gain time.

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
side-agnostic (`body_part: LeftArm` routes it into the Arm pool). When a wound rolls
`RightArm`, `RolledInjury` is stamped with `part: RightArm` — so the `DisableHand`
disables the RIGHT hand, even though the def was authored with `LeftArm`. The def's
`body_part` only routes the pool; the struck side is authoritative.

---

### Example B — `MovementCostMul` (GTW-444)

`MovementCostMul` is a **multiplicative, accumulated** effect (the "Hampered" status).
It slows movement by multiplying the terrain per-step floor TU cost.

**RON form:** `MovementCostMul(1.5)` — a bare `f32` in parentheses, parsed via
`#[serde(transparent)]` on `MovementCostFactor`.

**Payload newtype:** `MovementCostFactor(f32)` in `effect.rs`. `>= 1.0` means slower;
`1.0` is the identity (no slowdown). CANNOT derive `Eq` (inner is `f32`).

**Gain arm (`ledger.rs`):** `InjuryEffect::MovementCostMul(factor) => { self.movement = self.movement.times(factor); }`

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

# Armor Authoring Guide

How to create, extend, and maintain the armor roster — from a new `.armor.ron`
file to adding a new field to `ArmorSpec` end-to-end. This guide documents the
**current, landed state** of the armor system (GTW-269 / GTW-322 / GTW-323 /
GTW-374) and is the primary reference for content authors and engineers extending
armor mechanics.

---

## Part 1 — Creating a new armor suit (content authoring)

### 1a. Where the `.ron` file goes

Armor files live under `assets/content/armor/` — one file per suit, flat (no
subfolders). The file is named `<key>.armor.ron`, where `<key>` is the armor's
registry key.

**Key convention:** the file stem minus the `.armor` infix is the armor's
registry key. For example, `flak_vest.armor.ron` → key `"flak_vest"`. The key
must be unique across the folder.

**`ArmorName`:** the armor's name is NOT authored as a field in the `.ron` file
— it is the registry KEY (the file stem), supplied by the loader. Do not add a
`name:` field.

### 1b. The `.armor.ron` schema — every field with per-line comments

An armor suit is six `ArmorPiece` records, one per body location, each with
five stats. Follow the per-line-comment convention:

```ron
// Flak vest — the baseline light combat suit every ganger trusts.
// KEY = file stem minus `.armor` => `flak_vest`
(
    head:      (floor: 2, protection: 3, integrity: 50, hardness: 1, armor_type: Flak),
    // head is rarely struck but severity-amplifying when it is
    torso:     (floor: 2, protection: 4, integrity: 60, hardness: 1, armor_type: Flak),
    // torso covers the bulk of the silhouette — most hits land here
    left_arm:  (floor: 1, protection: 2, integrity: 45, hardness: 1, armor_type: Flak),
    right_arm: (floor: 1, protection: 2, integrity: 45, hardness: 1, armor_type: Flak),
    left_leg:  (floor: 1, protection: 2, integrity: 50, hardness: 1, armor_type: Flak),
    right_leg: (floor: 1, protection: 2, integrity: 50, hardness: 1, armor_type: Flak),
)
```

**Top-level field reference:**

| Field | Rust type | Notes |
|-------|-----------|-------|
| `head` | `ArmorPiece` | The head location — rarely struck, severity-amplifying |
| `torso` | `ArmorPiece` | The torso — most hits land here |
| `left_arm` | `ArmorPiece` | Left arm |
| `right_arm` | `ArmorPiece` | Right arm |
| `left_leg` | `ArmorPiece` | Left leg |
| `right_leg` | `ArmorPiece` | Right leg |

The six fields follow `BodyPart::ALL` order (Head, Torso, LeftArm, RightArm,
LeftLeg, RightLeg) — the canonical iteration order for per-location lookups.

### 1c. `ArmorPiece` — per-location stats

Each body location is an `ArmorPiece` struct with five fields:

```ron
(floor: 2, protection: 4, integrity: 60, hardness: 1, armor_type: Flak)
```

| Sub-field | Rust type | RON form | Semantics |
|-----------|-----------|----------|-----------|
| `floor` | `ArmorFloor` (`i32`) | bare integer | Minimum damage a landing hit deals through this armor — "a vest still bruises". Formula step 2 clamps up to this value. |
| `protection` | `ArmorProtection` (`i32`) | bare integer | Damage this armor soaks, down to `floor`. Formula: `dmg = max(floor, damage − max(0, protection − effPen))`. |
| `integrity` | `ArmorIntegrity` (`i32`) | bare integer | Durability — how much the armor can block before it is useless. Degrades per hit; at `integrity ≤ 0` the armor stops protecting. |
| `hardness` | `ArmorHardness` (`i32`) | bare integer | How much punch (weapon penetration) this armor ignores: `effPen = max(0, punch − hardness)`. Hardness does NOT degrade — shred attacks integrity, not hardness. |
| `armor_type` | `ArmorType` | enum variant | The armor's matchup-wheel node (see below). |

### 1d. Armor type vocabulary

`armor_type:` names one of the seven matchup-wheel nodes (`docs/combat/matchup.md`),
the armor half of the dual vocabulary:

| Variant | Wheel node | Mirror damage type |
|---------|-----------|-------------------|
| `Plated` (default) | 0 | `Shock` |
| `Refractive` | 1 | `Blast` |
| `Flak` | 2 | `Chem` |
| `Void` | 3 | `Kinetic` |
| `Hazard` | 4 | `Plasma` |
| `Reinforced` | 5 | `Rend` |
| `Ceramic` | 6 | `Las` |

The matchup wheel (`docs/combat/matchup.md`) determines which damage type
penetrates which armor type. `Flak` is the baseline light-armor type — the
reference budget the matchup wheel is measured against.

### 1e. Damage formula overview

Per-hit formula (from `docs/combat/weapons-and-armor.md`):

```
step 1: effPen = max(0, weapon.punch − armor.hardness)
step 2: dmg    = max(armor.floor, weapon.damage − max(0, armor.protection − effPen))
step 3: integrity -= min(armor.protection, weapon.damage) + effPen + weapon.shred
```

At `integrity ≤ 0` the armor stops applying (protection reads as 0 for the
remainder of the battle).

### 1f. The wear/damage signal surface (what the game shows)

Per-hit wear classifies into exactly ONE of three outcomes
(`ArmorWearOutcome`, `crates/gdtf_battle_sim/src/equipment/armor_wear/wear.rs`),
so at most one signal fires per hit:

| Outcome | Signal message | When |
|---------|----------------|------|
| `Damaged` | `ArmorDamaged` | The piece was reduced (`delta > 0`) and still protects — fires `0..n` times before the break; the presenter's "Armor -N" pop |
| `Broke` | `ArmorBroken` | THE protecting→broken crossing (`integrity ≤ 0`) — fires exactly once per piece |
| `Unaffected` | (none) | Already-broken / bare-flesh piece, or a zero-wear hit — never a misleading "Armor -0" |

Both are buffered messages the presenter consumes: `ArmorDamaged` draws the
"Armor -N" floating pop; `ArmorBroken` pops via the `ArmorBrokenFct` family
AND gains a combat-log line ([fct-authoring.md](fct-authoring.md) /
[combat-log-authoring.md](combat-log-authoring.md)). Nothing here is
authored — it is the runtime contract an armor author balances against
(higher `integrity` = more `ArmorDamaged` hits before the one `ArmorBroken`).

---

## Part 2 — How to extend the armor model

This section is for engineers adding a new per-location stat or suit-wide field.

### Step 1 — Decide where the new field lives

There are two extension points:

- **Per-location stat** (varies by body part): add a field to `ArmorPiece` in
  `stats.rs`.
- **Suit-wide field** (same for all locations): add a field to `ArmorSpec` in
  `spec.rs`.

### Step 2 — Add the newtype (if needed)

No bare types (no-bare-types rule). If the new field is a domain value, define a
newtype in `stats.rs` or the appropriate module. Follow the house style: private
inner, derived `Deref`, `#[serde(transparent)]`, doc comment with the why.

### Step 3 — Add the field to `ArmorPiece` or `ArmorSpec`

Files:

- `crates/gdtf_battle_sim/src/equipment/armor/stats.rs` — `ArmorPiece` and all
  armor-stat newtypes (`ArmorFloor`, `ArmorProtection`, `ArmorIntegrity`,
  `ArmorHardness`, `ArmorType`, `BodyPart`).
- `crates/gdtf_battle_sim/src/equipment/armor/spec.rs` — `ArmorSpec` (the six
  per-location `ArmorPiece` fields + any suit-wide fields).

`ArmorSpec` derives `Deserialize`, so the new field must also derive
`Deserialize`. A new required field breaks all existing `.armor.ron` files —
add a default (`#[serde(default)]`) or update every file in
`assets/content/armor/`.

### Step 4 — Thread through the wear and projection path

Armor stats that change per-hit flow through:

- `crates/gdtf_battle_sim/src/equipment/armor_wear/wear.rs` — `wear_armor` mutates
  `ArmorIntegrity`. If a new stat degrades, add the wear here.
- `crates/gdtf_battle_sim/src/combatants/ganger/rederive.rs` — the stat
  projector. If the new field modifies a derived stat, add the fold.

### Step 5 — Update test fixtures

Test fixtures that build `ArmorSpec` inline will need the new field. Update
any sim-side test support (the bespoke per-family loader test submodule was
retired with the GTW-570 generic content-family loader).

### Step 6 — Update authoring docs

Add the new field to the `ArmorPiece` field table and the RON example in this
guide. Run `cargo doc --workspace --no-deps` and confirm broken intra-doc links
are clean.

---

## Part 3 — Hot-reload

The armor system supports **live hot-reload** (GTW-374 pattern): editing any
`assets/content/armor/*.armor.ron` file while the game is running triggers
the generic `redrive_content_family::<ArmorFamily>` system (GTW-570) in
`crates/gdtf_assets/src/family/systems.rs`, which rebuilds the entire
`ArmorRegistry` from the persistent generic `ContentFolderHandle<ArmorFamily>`
(the family marker lives in `crates/gdtf_content_families/src/armor.rs`).

The rebuilt registry is written via `ResMut<ArmorRegistry>`, marking it
changed. The next battle setup resolves against the edited specs without a
restart. An `info!` line is emitted naming the reload.

---

## Part 4 — Loader and key resolution

Loader: the generic content-family resolve (GTW-570) in
`crates/gdtf_assets/src/family/systems.rs`, instantiated by the `ArmorFamily`
marker in `crates/gdtf_content_families/src/armor.rs` and registered with one
`register_content_family::<ArmorFamily>()` call in the Load plugin.

The loader:

1. Gates on `assets/content/armor/` loading (`RecursiveDependencyLoadState::Loaded`).
2. Reads each member handle as `RonAsset<ArmorSpec>`.
3. Keys it by the file stem with the `.armor` infix stripped:
   `flak_vest.armor.ron` → key `"flak_vest"`.
4. Inserts every `(ArmorName, ArmorSpec)` into the `ArmorRegistry`.

On failure (bad folder) it inserts an EMPTY `ArmorRegistry` so `Load` always
exits with one present (ADR-0003 safety-net); a battle then fails closed on a
missing armor key rather than crashing.

Key Rust types (all in `crates/gdtf_battle_sim/src/equipment/armor/`):

- `ArmorSpec` — `spec.rs`
- `ArmorPiece`, `ArmorFloor`, `ArmorProtection`, `ArmorIntegrity`,
  `ArmorHardness`, `ArmorType`, `BodyPart` — `stats.rs`
- `ArmorRegistry`, `ArmorName` — `registry.rs` (re-exported via `mod.rs`)

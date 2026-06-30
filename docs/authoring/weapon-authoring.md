# Weapon Authoring Guide

How to create, extend, and maintain the weapon roster — from a new
`.weapon.ron` file to adding a brand-new field to `WeaponSpec` end-to-end.
This guide documents the **current, landed state** of the weapon system
(GTW-257 / GTW-260 / GTW-374 / GTW-443) and is the primary reference for
content authors and engineers extending weapon mechanics.

---

## Part 1 — Creating a new weapon (content authoring)

### 1a. Where the `.ron` file goes

Ranged weapons live under `assets/content/weapons/ranged/` — one file per
weapon. The file is named `<key>.weapon.ron`, where `<key>` is the weapon's
registry key, the string a roster member references to load it. (GTW-505 split
the weapons tree into `ranged/` + `melee/`; melee weapons — the
`.melee_weapon.ron` sibling — are covered in
[combat/weapons-and-armor.md](../combat/weapons-and-armor.md) §Melee weapons.)

**Key convention:** the file stem minus the `.weapon` infix is the weapon's
registry key. For example, `ranged/stub_pistol.weapon.ron` → key `"stub_pistol"`.
The key must be unique across the folder.

**`WeaponName`:** the weapon's human-facing display name is NOT authored as a
field in the `.ron` file — it is the registry KEY (the file stem), supplied by
the loader from the file's stem. Do not add a `name:` field.

### 1b. The `.weapon.ron` schema — every field with per-line comments

Follow the per-line-comment convention (`.ron-files-commented` project rule):

```ron
// Stub pistol — crude slug-thrower. One-handed; fires single, burst, or full-auto.
// KEY = file stem minus `.weapon` => `stub_pistol`
(
    base_spread:   0.10,    // intrinsic angular dispersion (radians) before situational multipliers
    accuracy:      1.0,     // concentration term: clusters in-cone draw toward center (may exceed 1.0)
    kickback:      0.05,    // per-round recoil added in a burst; widens the cone for each successive round
    fatal_bias:    0.0,     // severity-score addend consumed by E3 (resolution.md §6); 0 = neutral
    damage:        6,       // base damage a hit deals before armor (signed i32 for formula arithmetic)
    punch:         2,       // armor protection a hit ignores — penetration (signed i32)
    shred:         1,       // extra integrity damage per hit, wearing armor durability (signed i32)
    damage_type:   Kinetic, // matchup-wheel node: Shock|Blast|Chem|Kinetic|Plasma|Rend|Las
    magazine: (
        size:      12,      // round capacity — how many shots before a reload
        reload_tu: 12,      // flat TU cost of a reload act (per-weapon, authored here)
    ),
    fire_mode: [            // the modes this weapon offers, in authored selector order
        (kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1),   // one aimed round
        (kind: Burst,  cone_mult: 1.3, tu_percent: 0.50, shots: 3),   // three-round burst, wider cone
        (kind: Full,   cone_mult: 1.7, tu_percent: 0.60, shots: 6),   // full-auto, sloppiest
    ],
    stable:        false,   // true = brace bonus unconditional (bipod/heavy piece); false = normal
    handedness:    OneHanded, // OneHanded (pistol) | TwoHanded (long-arm/heavy); default OneHanded
)
```

**Field reference:**

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `base_spread` | `BaseSpread` | bare `f32` | Intrinsic cone half-angle in radians |
| `accuracy` | `Accuracy` | bare `f32` | In-cone concentration term; may exceed 1.0 |
| `kickback` | `Kickback` | bare `f32` | Per-round recoil in a burst (0 = no recoil buildup) |
| `fatal_bias` | `FatalBias` | bare `f32` | Severity-score addend; 0.0 is neutral |
| `damage` | `WeaponDamage` | bare `i32` | Base damage before armor |
| `punch` | `WeaponPunch` | bare `i32` | Armor penetration (ignores this much protection) |
| `shred` | `WeaponShred` | bare `i32` | Extra armor integrity wear per hit |
| `damage_type` | `DamageType` | enum variant | One of the seven matchup-wheel nodes |
| `magazine` | `Magazine` | `(size: u16, reload_tu: u8)` | Capacity + reload cost (see below) |
| `fire_mode` | `FireMode` | list of `FireModeSpec` | The offered modes in selector order (see below) |
| `stable` | `Stable` | bare `bool` | `true` = unconditional brace bonus |
| `handedness` | `Handedness` | enum variant | `OneHanded` or `TwoHanded` |

### 1c. Magazine authoring

The `magazine:` field is a grouped struct with two authored leaves:

| Sub-field | Type | Notes |
|-----------|------|-------|
| `size` | `MagazineSize` (`u16`) | Round capacity before a reload |
| `reload_tu` | `ReloadTu` (`u8`) | Flat TU cost of the reload act |

The `rounds` (loaded count) leaf is NOT authored — it defaults to `0` at
parse time and is overwritten by `WeaponSpec::into_bundle`, which always spawns
the weapon FULL (`loaded == size`).

### 1d. Fire-mode authoring

The `fire_mode:` field is a **RON list** of `FireModeSpec` entries. A weapon
may offer any non-empty subset of `{Single, Burst, Full}` in authored order.
Convention: `Single` first; the selector shows modes in the order authored.

Each mode entry:

```ron
(kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1)
```

| Sub-field | Type | Notes |
|-----------|------|-------|
| `kind` | `ModeKind` | `Single` \| `Burst` \| `Full` (closed set) |
| `cone_mult` | `ModeConeMult` (`f32`) | Multiplier on the cone's angular size for this mode (single ≈ 1.0) |
| `tu_percent` | `ModeTuPercent` (`f32`) | Fraction of the shooter's TU pool the shot costs |
| `shots` | `ModeShots` (`u16`) | Rounds fired per shot action (Single = 1; Burst > 1) |

The human-facing label (`"single"` / `"burst"` / `"full-auto"`) is derived from
`ModeKind`'s `Display` impl — it is NOT a stored string and is NOT authored.

A weapon with no authored modes is invalid at runtime (the code is defensive —
`FireMode::single()` has a structural fallback — but every real weapon must
list at least one mode).

### 1e. Damage type vocabulary

`damage_type:` names one of the seven matchup-wheel nodes (`docs/combat/matchup.md`):

| Variant | Wheel node | Theme |
|---------|-----------|-------|
| `Shock` | 0 | Arc / EMP |
| `Blast` | 1 | Explosives / concussion |
| `Chem` | 2 | Toxin / acid / gas |
| `Kinetic` | 3 | Slugs / autoguns / shrapnel |
| `Plasma` | 4 | Superheated |
| `Rend` | 5 | Chain / power edges |
| `Las` | 6 | Beams |

### 1f. `stable` — brace bonus

`stable: true` means the weapon is braced-by-design (a bipod-mounted or
inherently-steady heavy piece). This engages the §1a brace bonus
UNCONDITIONALLY, bypassing the normal stance/cover-height gate. Most weapons
are `false`.

### 1g. `handedness` — GTW-443

`handedness:` declares how many hands the weapon requires to fire:

| Variant | Meaning |
|---------|---------|
| `OneHanded` (default) | Fires with a single working hand (pistols, sidearms) |
| `TwoHanded` | Needs BOTH hands; refused when `hands_available() < 2` |

A ganger with a `DisableHand` injury cannot fire a `TwoHanded` weapon. Omitting
the field is NOT supported — it must be authored explicitly.

---

## Part 2 — How to extend the weapon model

This section is for engineers adding a new field, stat, or mechanic to the
weapon system. Follow these steps exactly.

### Step 1 — Add the field to `WeaponSpec`

File: `crates/gdtf_battle_sim/src/equipment/weapon/spec.rs`

Add a new `pub` field to `WeaponSpec`. If the payload is a domain value, define
a newtype first (no-bare-types rule). `WeaponSpec` derives `Deserialize`, so
the new field must also derive `Deserialize`; add `#[serde(transparent)]` on the
newtype so it parses as a bare RON scalar.

### Step 2 — Thread through `into_bundle` and `WeaponBundle`

`WeaponSpec::into_bundle` (`spec.rs`) constructs a `WeaponBundle` from the
spec fields. If the new field becomes a component on the armed entity, add it to
`DamageProfile::new` / `HandlingProfile::new` or add it directly to
`WeaponBundle::new`.

File: `crates/gdtf_battle_sim/src/equipment/weapon/bundle.rs`

### Step 3 — Define the component (if it lives on the armed entity)

Add a `#[derive(Component)]` newtype to `components.rs`:

File: `crates/gdtf_battle_sim/src/equipment/weapon/components.rs`

Follow the house style: private inner, derived `Deref`, `#[serde(transparent)]`,
doc comment with the why. Derive `Default` ONLY as a `bsn!` spawn-seed sentinel
— note that explicitly in the doc comment. Do not derive `Default` as a
meaningful authoring value.

### Step 4 — Update the fire-eligibility gate (if it restricts firing)

If the new field gates when a weapon can fire (like `Handedness` gates on
`hands_available()`), add the check to the `can_fire` guard in the fire system.

### Step 5 — Update the weapon tuning tests and fixtures

Test fixtures in `crates/gdtf_battle_sim/src/equipment/weapon/test/` use
inline RON strings to build `WeaponSpec`. A new required field will break those
fixtures — add the field to every inline RON in the test support file
(`test/support.rs`).

### Step 6 — Update authoring docs

Add the new field to the field table and the RON example in this guide. Run
`cargo doc --workspace --no-deps` and confirm broken intra-doc links are clean.

---

## Part 3 — Hot-reload

The weapon system supports **live hot-reload** (GTW-374 pattern): editing any
`assets/content/weapons/ranged/*.weapon.ron` file while the game is running triggers
`redrive_weapons_on_asset_event` in
`crates/gdtf_app/src/states/load/systems/resolve/weapons.rs`, which rebuilds
the entire `WeaponRegistry` from the persistent `ActiveWeaponsFolderHandle`.

The rebuilt registry is written via `ResMut<WeaponRegistry>`, marking it
changed. The next battle setup resolves against the edited specs without a
restart. An `info!` line is emitted naming the reload.

---

## Part 4 — Loader and key resolution

Loader: `crates/gdtf_app/src/states/load/systems/resolve/weapons.rs`

The loader:

1. Gates on `assets/content/weapons/ranged/` loading (`RecursiveDependencyLoadState::Loaded`).
2. Reads each member handle as `RonAsset<WeaponSpec>`.
3. Keys it by the file stem with the `.weapon` infix stripped:
   `stub_pistol.weapon.ron` → key `"stub_pistol"`.
4. Inserts every `(WeaponName, WeaponSpec)` into the `WeaponRegistry`.

On failure (bad folder) it inserts an EMPTY `WeaponRegistry` so `Load` always
exits with one present (ADR-0003 safety-net); a battle then fails closed with
`WeaponNotFound` rather than crashing.

Key Rust types (all in `crates/gdtf_battle_sim/src/equipment/weapon/`):

- `WeaponSpec` — `spec.rs`
- `WeaponBundle` — `bundle.rs`
- `BaseSpread`, `Accuracy`, `Kickback`, `FatalBias`, `WeaponDamage`,
  `WeaponPunch`, `WeaponShred`, `DamageType`, `Stable`, `Handedness`,
  `WeaponName`, `Weapon` — `components.rs`
- `FireMode`, `FireModeSpec`, `ModeKind`, `ModeConeMult`, `ModeTuPercent`,
  `ModeShots` — `fire_mode.rs`
- `Magazine`, `ReloadTu`, `LoadedRounds` — `crates/gdtf_battle_sim/src/equipment/magazine/ammo.rs`
- `WeaponRegistry` — `registry.rs` (re-exported via `mod.rs`)

# Weapon Authoring Guide

How to create, extend, and maintain the weapon roster — from a new
`.weapon.ron` file to adding a brand-new field to `WeaponSpec` end-to-end.
This guide documents the **current, landed state** of the weapon system
(/ / /, plus the optional-field additions
 / / / / /) and is the primary
reference for content authors and engineers extending weapon mechanics.
Melee weapons are Part 5.

---

## Part 1 — Creating a new weapon (content authoring)

### 1a. Where the `.ron` file goes

Ranged weapons live under `assets/content/weapons/ranged/` — one file per
weapon. The file is named `<key>.weapon.ron`, where `<key>` is the weapon's
registry key, the string a roster member references to load it. (split
the weapons tree into `ranged/` + `melee/`; melee weapons — the
`.melee_weapon.ron` sibling — are covered in Part 5 below.)

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
    // ---- optional fields (all #[serde(default)]; omit them for a plain gun) ----
    // shove: true, // knockback tag
    // trajectory: Arc, // lobbed grenade parabola
    // slots: [(Muzzle, 1), (Sight, 1)], // offered attachment slots
    // attachments: ["suppressor"], // fitted attachment item keys
    // dot: Some((damage: 4, damage_type: Plasma, turns: 3)), //
    // on_death: Some(Explode(hit_type: Blast(radius: 1), damage: 8, damage_type: Blast)), //
)
```

**Field reference** (one row per `WeaponSpec` field, in declaration order —
`crates/gdtf_battle_sim/src/equipment/weapon/spec.rs`):

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
| `shove` | `Shove` | bare `bool`, optional | knockback tag; omitted = `false` (see 1h) |
| `handedness` | `Handedness` | enum variant | `OneHanded` or `TwoHanded` |
| `trajectory` | `TrajectoryStyle` | enum variant, optional | `Straight` (default) or `Arc` (see 1i) |
| `slots` | `WeaponSlots` | pair list, optional | Offered attachment slots + capacities; omitted = none fit (see 1j) |
| `attachments` | `FittedAttachments` | string list, optional | Fitted attachment ITEM keys; omitted = `[]` (see 1j) |
| `dot` | `Option<DotProfile>` | `Some((…))`, optional | Damage-over-time profile; omitted = `None` (see 1k) |
| `on_death` | `Option<OnDeathEffect>` | `Some(…)`, optional | Wielder-death effect; omitted = `None` (see 1l) |

### 1c. Magazine authoring

The `magazine:` field is a grouped struct with two authored leaves:

| Sub-field | Type | Notes |
|-----------|------|-------|
| `size` | `MagazineSize` (`u16`) | Round capacity before a reload |
| `reload_tu` | `ReloadTu` (`u8`) | Flat TU cost of the reload act |

The `rounds` (loaded count) leaf is NOT authored — it defaults to `0` at
parse time and is overwritten by `WeaponSpec::into_bundle`, which always spawns
the weapon FULL (`loaded == size`).

Reloading is a real, per-weapon TU-costed act: `reload_tu` is the flat cost
the reload act charges. A fitted `ReloadTime(scale)` attachment SCALES it
per-item (bidirectional — `< 1.0` faster, `> 1.0` slower; renamed the
effect from the speed-up-only `FastReload` misnomer) — see
[attachment-authoring.md](attachment-authoring.md) §1d.

### 1d. Fire-mode authoring

The `fire_mode:` field is a **RON list** of `FireModeSpec` entries. A weapon
may offer any non-empty subset of `{Single, Burst, Full}` in authored order.
Convention: `Single` first; the selector shows modes in the order authored.

The editor's Weapon form holds the same non-empty rule on its own path.
`WeaponDraft::can_remove_fire_mode` is what greys out its Remove button, and
`editor.list_op` refuses a `Remove` on `WeaponFireModes` through the same method
while one mode remains.

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

### 1g. `handedness` —

`handedness:` declares how many hands the weapon requires to fire:

| Variant | Meaning |
|---------|---------|
| `OneHanded` (default) | Fires with a single working hand (pistols, sidearms) |
| `TwoHanded` | Needs BOTH hands; refused when `hands_available() < 2` |

A ganger with a `DisableHand` injury cannot fire a `TwoHanded` weapon. Omitting
the field is NOT supported — it must be authored explicitly.

### 1h. `shove` — knockback tag (optional)

`shove: true` knocks the target back one cell on a connecting shot, in
addition to the shot's damage. `#[serde(default)]` — an omitted field is a
non-shove weapon (`false`).

### 1i. `trajectory` — flat ray or lobbed arc (optional)

`trajectory:` is the `TrajectoryStyle` enum: `Straight` (a flat ray — the
default when omitted) or `Arc` (a lobbed grenade parabola with no LOS gate).
Grenades and grenade launchers author `trajectory: Arc`.

### 1j. `slots` and `attachments` — the attachment fields (optional)

`slots:` declares WHICH attachment slots the weapon offers and how many
attachments each holds, as a `(slot, capacity)` pair list, e.g.
`slots: [(Muzzle, 1), (Sight, 1), (Rail, 3)]`. The slot vocabulary is the
closed `AttachmentSlot` enum: `Muzzle` / `Sight` / `Rail` / `Magazine` (ranged)
and `Counterweight` / `Pommel` (melee) — class gating EMERGES from the declared
slots, never from a tag on the item. Only a magazine-fed gun declares a
`Magazine` slot; energy- and fluid-fed weapons omit it. An omitted
`slots:` field is the EMPTY
declaration: NO attachment fits (fail-closed — a thrown charge takes no
fittings).

`attachments:` lists the fitted attachment ITEM keys (each a
`assets/content/attachments/<key>.attachment.ron` file stem, e.g.
`attachments: ["suppressor"]`). At battle setup each key is resolved against
the `AttachmentRegistry` and admitted only into a declared slot with free
capacity; a non-fitting item is cleanly skipped. Attachment ITEMS themselves
(the `AttachmentSpec` + its typed effect list) are authored in
`assets/content/attachments/` and their effect behaviors live in the
attachments palette (`crates/gdtf_battle_sim/src/effects/attachments/`,
 — one file per effect) — the full item schema + effect vocabulary is
[attachment-authoring.md](attachment-authoring.md).

### 1k. `dot` — damage-over-time profile (optional)

`dot:` gives the weapon a damage-over-time profile a PENETRATING hit seeds on
the struck ganger:

```ron
dot: Some((
    damage:      4,       // HP per turn, bypasses armor
    damage_type: Plasma,  // presentation flavour only (no soak lookup)
    turns:       3,       // duration, MUST be >= 1; a second penetrating hit REFRESHES, never stacks
)),
```

`turns` must be **at least 1**. A zero-turn DOT makes no sense and is
unrepresentable (`DotTurns` wraps `NonZeroU8`).

The two paths that reach `turns` answer a zero differently. Loading a **file**
that authors `turns: 0` FAILS: the per-file salvage rejects it loudly as a
`MalformedFile` finding on the `ContentIntegrityReport` and the weapon never
enters the registry (no silent clamp-to-1). Editing the **form**
cannot reach zero at all: its drag is ranged `1..=u8::MAX`, and it commits through
`weapon_form::dot_turns_from_raw`, which turns a 0 into a 1. `editor.set_field`
writes `WeaponDotTurns` through that same function, so a 0 sent over the wire
answers `Ran` and reads back as 1.

### 1l. `on_death` — wielder-death effect (optional)

`on_death:` names the `OnDeathEffect` the WIELDING ganger's death fans (a live
satchel charge, an unstable power cell): `Explode(hit_type: …, damage: …,
damage_type: …)` or `LeaveField(field: "<field key>")`. Example
(`assets/content/weapons/ranged/volatile_charge.weapon.ron`):

```ron
on_death: Some(Explode(
    hit_type:    Blast(radius: 1),  // AoE template (Blast / Cone / Line / Single)
    damage:      8,                 // flat HP drained per affected ganger (no RNG)
    damage_type: Blast,             // wheel-node flavour (the drain bypasses armor)
)),
```

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

Add a `#[derive(Component)]` newtype to the `components/` module, in the
concern leaf it belongs to (`ballistics.rs` / `damage.rs` / `handling.rs` /
`markers.rs` / `tags.rs`):

File: `crates/gdtf_battle_sim/src/equipment/weapon/components/`

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

The weapon system supports **live hot-reload** (pattern): editing any
`assets/content/weapons/ranged/*.weapon.ron` file while the game is running triggers
the generic `redrive_content_family::<WeaponsFamily>` system in
`crates/gdtf_assets/src/family/systems.rs`, which rebuilds the entire
`WeaponRegistry` from the persistent generic `ContentFolderHandle<WeaponsFamily>`
(the family marker lives in `crates/gdtf_content_families/src/weapons.rs`).

The rebuilt registry is written via `ResMut<WeaponRegistry>`, marking it
changed. The next battle setup resolves against the edited specs without a
restart. An `info!` line is emitted naming the reload.

---

## Part 4 — Loader and key resolution

Loader: the generic content-family resolve in
`crates/gdtf_assets/src/family/systems.rs`, instantiated by the `WeaponsFamily`
marker in `crates/gdtf_content_families/src/weapons.rs` and registered with one
`register_content_family::<WeaponsFamily>()` call in the Load plugin.

The loader:

1. Gates on `assets/content/weapons/ranged/` loading (`RecursiveDependencyLoadState::Loaded`).
2. Reads each member handle as `RonAsset<WeaponSpec>`.
3. Keys it by the file stem with the `.weapon` infix stripped:
   `stub_pistol.weapon.ron` → key `"stub_pistol"`.
4. Inserts every `(WeaponName, WeaponSpec)` into the `WeaponRegistry`.

On failure (bad folder) it inserts an EMPTY `WeaponRegistry` so `Load` always
exits with one present (safety net); a battle then fails closed with
`WeaponNotFound` rather than crashing.

Key Rust types (all in `crates/gdtf_battle_sim/src/equipment/weapon/`):

- `WeaponSpec` — `spec.rs`
- `WeaponBundle` — `bundle.rs`
- `BaseSpread`, `Accuracy`, `Kickback`, `FatalBias`, `WeaponDamage`,
  `WeaponPunch`, `WeaponShred`, `DamageType`, `Stable`, `Handedness`,
  `WeaponName`, `Weapon` — `components/` (one concern leaf each)
- `FireMode`, `FireModeSpec`, `ModeKind`, `ModeConeMult`, `ModeTuPercent`,
  `ModeShots` — `fire_mode.rs`
- `Magazine`, `ReloadTu`, `LoadedRounds` — `crates/gdtf_battle_sim/src/equipment/magazine/ammo.rs`
- `WeaponRegistry` — `registry.rs` (re-exported via `mod.rs`)

---

## Part 5 — Melee weapons (`.melee_weapon.ron`)

Melee weapons (child) live under
`assets/content/weapons/melee/` — one file per weapon, named
`<key>.melee_weapon.ron` (the stem is the registry key; no `name:` field, the
ranged rule). They SHARE the ranged damage model verbatim and swap the
cone/handling group for melee-only mechanics.

### 5a. The schema — by example

The shipped `assets/content/weapons/melee/fists.melee_weapon.ron` (the unarmed
default — magnitudes are tuning DATA, never pinned):

```ron
(
    // SHARED ranged damage model — a strike resolves through the SAME per-hit formula:
    damage:        4,        // modest blunt damage
    punch:         0,        // bare knuckles ignore no armor
    shred:         0,        // no armor-durability chew
    damage_type:   Kinetic,  // blunt impact — the Kinetic wheel node
    fatal_bias:    0.0,      // no severity push
    handedness:    OneHanded,// a punch needs one working hand
    // MELEE-ONLY mechanics:
    reach:         1,        // strikes the adjacent cell (optional; defaults to 1)
    fight_mode: [
        (kind: Swing, tu_cost: 20, strikes: 1),  // flat TU cost + strike count per mode
    ],
    // slots: / attachments: omitted — bare hands take no fittings (fail-closed)
)
```

**Field reference** (`MeleeWeaponSpec`,
`crates/gdtf_battle_sim/src/equipment/weapon/melee/spec.rs`): `damage`,
`punch`, `shred`, `damage_type`, `fatal_bias`, `handedness`, and the optional
`shove` are the SAME newtypes and semantics as the ranged table in §1b —
there is deliberately no melee mirror vocabulary. The melee-only fields:

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `reach` | `Reach` | bare integer, optional | Cells away a strike can land. `#[serde(default)]` → `1` |
| `fight_mode` | `FightMode` | list of `FightModeSpec` | The offered fight modes: `kind` (`Swing` \| `Thrust`), `tu_cost` (flat TU), `strikes` (count) |
| `slots` / `attachments` | `WeaponSlots` / `FittedAttachments` | as ranged §1j, optional | Melee weapons get FULL attachment support — melee slots are `Counterweight` / `Pommel` |

Unlike a ranged fire mode (a TU *percentage*), a fight mode charges a FLAT
`tu_cost`. The mode label (`"swing"` / `"thrust"`) derives from
`FightModeKind`'s `Display` — never authored.

### 5b. The fists default

Every ganger can melee: a gang member that authors no `melee_weapon:` key
resolves to the shipped `"fists"` registry key at setup (`FISTS_KEY`,
`crates/gdtf_battle_sim/src/equipment/weapon/melee/registry.rs`) — so
`fists.melee_weapon.ron` must always exist. See
[battlefield-authoring.md](battlefield-authoring.md) §4a for the gang-side
field.

### 5c. Loading, hot-reload, verify

Melee weapons are their own stem-keyed content family (`MeleeWeaponsFamily`,
`crates/gdtf_content_families/src/melee_weapons.rs` — folder
`content/weapons/melee`, extension `melee_weapon.ron`) resolving into the
`MeleeWeaponRegistry`; everything in Parts 3–4 (hot-reload, salvage,
fail-closed empty registry) applies unchanged
([content-families.md](content-families.md)). Load coverage:
`crates/gdtf_app/tests/load_melee_weapons.rs`. In game (`cargo drun`), the
contextual MELEE button offers an adjacent-enemy strike; a ganger with no
authored melee key punches with fists.

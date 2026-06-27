# Terrain Authoring Guide

How to create, extend, and maintain the terrain piece roster — from a new
`.terrain.ron` file to adding a new piece kind end-to-end. This guide documents
the **current, landed state** of the terrain system (GTW-394 / GTW-396) and is
the primary reference for content authors and engineers extending terrain mechanics.

---

## Part 1 — Creating a new terrain piece (content authoring)

### 1a. Where the `.ron` file goes

Terrain files live under `assets/content/terrain/` — one file per piece type,
flat (no subfolders). The file is named `<key>.terrain.ron`, where `<key>` is
the piece's registry key, the string a generated cell references to select its
terrain type.

**Key convention:** the file stem minus the `.terrain` infix is the terrain's
registry key. For example, `deck_floor.terrain.ron` → key `"deck_floor"`. The
key must be unique across the folder.

**`TerrainName`:** the piece type's name is NOT a separate authored field — it
is the registry KEY, supplied by the loader from the file stem.

### 1b. The shared header — `graphic` and `footfall`

Every terrain piece, regardless of kind, carries two shared presentation-hook
fields:

| Field | Rust type | Notes |
|-------|-----------|-------|
| `graphic` | `TerrainGraphicKey` (`String`) | An opaque key the presenter resolves via `TileRoles` to an atlas tile. The sim stores it render-free and never resolves it. |
| `footfall` | `FootfallSound` (`String`) | An opaque key the presenter resolves to an audio clip. No audio system is built yet; carried for the future footfall-audio pass without a schema change. Use `"none"` for pieces with no walkable surface (walls, cover props). |

The presenter resolves `graphic:` against the `alt_tileset_terrain` atlas via
`TileRoles`; the sim is render-free and never touches atlas indices.

### 1c. The `kind:` payload — five piece types

The `kind:` field selects which of the five piece types this terrain is and
carries its kind-specific stats:

| `kind:` variant | Walkable | Destructible | Has height band |
|-----------------|---------|--------------|----------------|
| `Floor(…)` | Yes | No | No |
| `Wall(…)` | No | Yes | Yes |
| `Cover(…)` | No | Yes | Yes |
| `Scatter(…)` | No | Yes | Yes |
| `Slab(…)` | No | Yes | **No** |

**Key design note:** a `Slab` has NO height band — it spans the whole
z-boundary; the march stops on an intact slab regardless of the shot's band
(`docs/combat/resolution.md §2`). Cover/Wall/Scatter have a `height_band:` because
a round must fly strictly HIGHER than the band to clear them (`resolution.md §3`).
Do not add `height_band:` to a `Slab` — it is not part of the schema.

---

### 1d. `Floor` — walkable ground

A `Floor` carries only a move cost; it has no HP or armor (you do not shoot the
floor you stand on).

```ron
// Deck floor — default walkable plating.
// KEY = deck_floor
(
    graphic:  "floor",           // presenter TileRoles key -> alt_tileset_terrain atlas
    footfall: "footfall_metal",  // footfall sound key
    kind: Floor((
        move_cost: 4,            // TU to step onto this floor (tunable)
    )),
)
```

`Floor` stats:

| Sub-field | Rust type | RON form | Notes |
|-----------|-----------|----------|-------|
| `move_cost` | `MoveCost` (`u8`) | bare integer | TU cost to step onto this floor type. Uses the `MoveCost` newtype from `crate::tuning`. |

---

### 1e. `Wall` — solid blocking geometry

A `Wall` is a destructible structural piece. A wall fills the cell; nothing
flies over it within a storey at any band.

```ron
// Bulkhead wall — solid blocking geometry.
// KEY = bulkhead_wall
(
    graphic:  "wall",
    footfall: "footfall_metal",  // walls have no walkable surface; "none" is also valid
    kind: Wall((
        max_hp:           40,    // structural HP pool (CoverHp; tunable)
        armor_protection: 6,     // damage soak — same armor model as a ganger
        armor_hardness:   3,     // punch this wall shrugs off
        height_band:      High,  // Low | Mid | High — a wall fills the cell; High = nothing clears it
    )),
)
```

---

### 1f. `Cover` — chest-high or low cover props

A `Cover` piece is a destructible structural piece a round may fly over if its
shot band exceeds the piece's `height_band` (`resolution.md §3`).

```ron
// Barricade — chest-high scatter prop.
// KEY = barricade
(
    graphic:  "cover",
    footfall: "none",            // cover props have no walkable surface
    kind: Cover((
        max_hp:           30,    // structural HP (CoverHp; tunable)
        armor_protection: 2,     // damage soak
        armor_hardness:   1,     // penetration shrugged off
        height_band:      Low,   // Low | Mid | High; Low = a Mid/High round clears it
    )),
)
```

---

### 1g. `Scatter` — loose debris / props

A `Scatter` piece is a destructible structural piece (typically low and fragile)
that shares the structural model with Wall/Cover.

```ron
// Debris pile — loose scatter.
// KEY = debris_pile
(
    graphic:  "rubble",
    footfall: "footfall_rubble",
    kind: Scatter((
        max_hp:           8,     // CoverHp (scatter is fragile; tunable)
        armor_protection: 1,
        armor_hardness:   0,     // no punch resistance
        height_band:      Low,
    )),
)
```

---

### 1h. `Slab` — floor/roof structural slab (NO height band)

A `Slab` is a destructible piece spanning a z-boundary. It carries NO
`height_band:` — a slab is not flown over, it is stood on or stood under. Its
HP pool uses the `SlabHp` newtype (distinct from `CoverHp` — the no-bare-types
rule: a slab HP pool is not a cover HP pool).

```ron
// Deck slab — a floor/roof structural slab spanning a z-boundary.
// KEY = deck_slab
(
    graphic:  "slab",
    footfall: "none",            // no audio system yet
    kind: Slab((
        max_hp:           120,   // SlabHp (distinct from CoverHp; tunable)
        armor_protection: 4,     // damage soak
        armor_hardness:   2,     // penetration shrugged off
        // NOTE: NO height_band — a slab has none (resolution.md §2)
    )),
)
```

---

### 1i. Structural spec field reference (`Wall`, `Cover`, `Scatter`)

`Wall`, `Cover`, and `Scatter` all share `StructuralSpec`:

| Sub-field | Rust type | RON form | Notes |
|-----------|-----------|----------|-------|
| `max_hp` | `CoverHp` | bare integer | Full structural HP the piece seeds to |
| `armor_protection` | `ArmorProtection` (`i32`) | bare integer | Damage soak — same armor model as a ganger |
| `armor_hardness` | `ArmorHardness` (`i32`) | bare integer | Penetration this piece shrugs off |
| `height_band` | `HeightBand` | `Low` \| `Mid` \| `High` | Clearance band; a round must fly strictly higher to clear |

`Slab` uses `SlabPieceSpec` (HP + armor, no band):

| Sub-field | Rust type | RON form | Notes |
|-----------|-----------|----------|-------|
| `max_hp` | `SlabHp` | bare integer | Full structural HP (distinct pool from `CoverHp`) |
| `armor_protection` | `ArmorProtection` (`i32`) | bare integer | Damage soak |
| `armor_hardness` | `ArmorHardness` (`i32`) | bare integer | Penetration shrugged off |

---

### 1j. Height band vocabulary

`height_band:` is the `HeightBand` enum (`Low` / `Mid` / `High`), used by
`resolution.md §3` to determine when a round clears a piece:

| Variant | Meaning |
|---------|---------|
| `Low` | Low cover — cleared by Mid or High round bands |
| `Mid` | Mid cover — cleared only by a High round band |
| `High` | Full wall height — nothing clears it within a storey |

---

## Part 2 — How to extend the terrain model

This section is for engineers adding a new piece kind or a new field to an
existing kind.

### Step 1 — Add a new variant to `TerrainKindSpec` (new kind only)

File: `crates/gdtf_battle_sim/src/terrain/piece/spec.rs`

Add a new variant to `TerrainKindSpec` with its payload struct (define the
payload struct in the same file). `TerrainKindSpec` derives `Deserialize`, so
the new variant's payload must also derive `Deserialize`. Use existing newtypes
for domain values (`CoverHp`, `ArmorProtection`, `ArmorHardness`, etc.) — no
bare types.

### Step 2 — Add a new field to an existing kind

For a new field on `FloorSpec`, `StructuralSpec`, or `SlabPieceSpec`: add the
field and its newtype in `spec.rs`. If the field is optional / backwards-compatible,
add `#[serde(default)]`. If required, update every `.terrain.ron` in
`assets/content/terrain/`.

### Step 3 — Thread through the sim

Terrain data is consumed at battle setup (the `terrain_resolve` path in
`crates/gdtf_battle_sim/src/lifecycle/situation/`) and by the pathfinder /
move-cost / shot-march systems. If a new field changes movement or blocking,
add it to the relevant system:

- Move cost: `crates/gdtf_battle_sim/src/acts_runtime/move_acts/` + the
  pathfinder (`crates/gdtf_battle_sim/src/perception/pathfinder/`).
- Blocking / LOS: `crates/gdtf_battle_sim/src/perception/visibility/` and
  `crates/gdtf_battle_sim/src/shot_pipeline/march/`.

### Step 4 — Update test fixtures

The terrain test fixture helpers in
`crates/gdtf_app/src/states/load/systems/resolve/terrain.rs` (the test submodule)
build `TerrainSpec` from inline RON. A new required field breaks them — add the
field or add `#[serde(default)]`.

### Step 5 — Update authoring docs

Add the new kind/field to the field tables and a commented `.ron` example in
this guide. Run `cargo doc --workspace --no-deps` and confirm broken intra-doc
links are clean.

---

## Part 3 — Hot-reload

The terrain system supports **live hot-reload** (GTW-394 pattern): editing any
`assets/content/terrain/*.terrain.ron` file while the game is running triggers
`redrive_terrain_on_asset_event` in
`crates/gdtf_app/src/states/load/systems/resolve/terrain.rs`, which rebuilds
the entire `TerrainRegistry` from the persistent `ActiveTerrainFolderHandle`.

The rebuilt registry is written via `ResMut<TerrainRegistry>`, marking it
changed. The next battle setup resolves against the edited specs without a
restart. An `info!` line is emitted naming the reload.

---

## Part 4 — Loader and key resolution

Loader: `crates/gdtf_app/src/states/load/systems/resolve/terrain.rs`

The loader:

1. Gates on `assets/content/terrain/` loading (`RecursiveDependencyLoadState::Loaded`).
2. Reads each member handle as `RonAsset<TerrainSpec>`.
3. Keys it by the file stem with the `.terrain` infix stripped:
   `deck_floor.terrain.ron` → key `"deck_floor"`.
4. Inserts every `(TerrainName, TerrainSpec)` into the `TerrainRegistry`.

On failure (bad folder) it inserts an EMPTY `TerrainRegistry` so `Load` always
exits with one present (ADR-0003 safety-net); a battle then fails closed on a
missing terrain key rather than crashing.

Key Rust types (all in `crates/gdtf_battle_sim/src/terrain/piece/`):

- `TerrainSpec`, `TerrainKindSpec`, `FloorSpec`, `StructuralSpec`,
  `SlabPieceSpec` — `spec.rs`
- `TerrainGraphicKey`, `FootfallSound` — `components.rs`
- `TerrainRegistry` — `registry.rs`; `TerrainName` — `components.rs` (both re-exported via `mod.rs`)

Supporting newtypes from other modules:

- `MoveCost` — `crates/gdtf_battle_sim/src/tuning/`
- `CoverHp`, `HeightBand` — `crates/gdtf_battle_sim/src/terrain/cover/`
- `SlabHp` — `crates/gdtf_battle_sim/src/terrain/slab/`
- `ArmorProtection`, `ArmorHardness` — `crates/gdtf_battle_sim/src/equipment/armor/stats.rs`

# Terrain Authoring Guide

How to create, extend, and maintain the terrain roster — from a new
`.terrain_def.ron` file to adding a new sim kind end-to-end. This guide
documents the **current, landed state** of the UUID-keyed terrain model
(GTW-484…496 redesign; asset roots unified in GTW-562; loader generalized in
GTW-570) and is the primary reference for content authors and engineers
extending terrain mechanics.

The historical flat, filename-keyed model (`*.terrain.ron` / `TerrainRegistry`)
is RETIRED — see
[terrain-content-migration.md](terrain-content-migration.md) for the migration
record.

---

## Part 0 — The unified authored-asset roots

Every authored data-family `.ron` in the game lives under **one root**:
`assets/content/` (GTW-556 consolidated maps/situations there; GTW-562 moved
terrain under it, completing the unification). Sprite art lives under
`assets/sprites/`. There are no other authored-content roots.

| Family | Folder | Extension |
|--------|--------|-----------|
| Ranged weapons | `assets/content/weapons/ranged/` | `.weapon.ron` |
| Melee weapons | `assets/content/weapons/melee/` | `.melee_weapon.ron` |
| Armor | `assets/content/armor/` | `.armor.ron` |
| Attachments | `assets/content/attachments/` | `.attachment.ron` |
| Fields | `assets/content/fields/` | `.field.ron` |
| Gangs | `assets/content/gangs/` | `.gang.ron` |
| Injuries | `assets/content/injuries/` | `.injury.ron` / `.weighting.ron` |
| Map prefabs | `assets/content/maps/<theme>/<size>/` | `.prefab.ron` |
| Situations | `assets/content/situations/` (single file — see below) | `.ron` |
| **Terrain defs** | `assets/content/terrain/<theme>/` | `.terrain_def.ron` |
| **Theme defs** | `assets/content/terrain/<theme>/` | `.terrain_theme.ron` |

Terrain and themes share ONE mixed per-theme tree — see Part 1. Every family
above EXCEPT Situations is recursively folder-loaded: do not drop a stray
non-`.ron` file into those folders — an unloadable member fails the whole
folder walk. Situations is the one exception: the loader reads exactly ONE
named file, `content/situations/skirmish.ron` (a single-file hot-RON load,
not a folder walk), so a new `.ron` dropped beside it loads nothing.

---

## Part 1 — Creating a new terrain piece (content authoring)

### 1a. Where the `.ron` file goes

Terrain files live under `assets/content/terrain/<theme>/` — one folder per
theme, holding that theme's pieces plus the theme file itself:

- `<piece>.terrain_def.ron` — one `TerrainDef` per file (the piece).
- `<theme>.terrain_theme.ron` — the theme's `UuidThemeDef` (its palette).

Both families share the same mixed `content/terrain/` tree; the loader tells
them apart by their dedicated compound extensions, so the extension must be
EXACTLY `.terrain_def.ron` / `.terrain_theme.ron`.

**Key convention:** a terrain def is **payload-keyed** — its stable key is the
`key:` UUID authored INSIDE the file (`TerrainUuid`), NOT the filename. The
filename stem is a human convenience only; renaming or moving the file changes
nothing. Mint a fresh v4 UUID for each new piece (any `uuidgen`; the code-side
equivalent is `TerrainUuid::generate()`).

**Theme membership:** a piece only participates in a theme when its UUID is
listed in that theme's `terrain:` palette (see 1f). A def whose UUID no palette
references still loads into the registry but is drawn from by nothing.

### 1b. The `TerrainDef` schema

Every `.terrain_def.ron` deserializes into `TerrainDef`
(`crates/gdtf_battle_sim/src/terrain/def/definition.rs`):

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `key` | `TerrainUuid` | UUID string | The stable key themes / prefabs / the registry reference. Required, unique. |
| `display_name` | `TerrainDisplayName` | bare string | Human label for tooling / the editor. |
| `sim_kind` | `TerrainSimKind` | struct variant | The SIM half — structural kind + combat stats (see 1c). |
| `presenter_kind` | `TerrainPresenterKind` | struct variant | The PRESENTER half — graphic role key (+ optional slab footfall; see 1d). |
| `tags` | `Vec<TerrainTag>` | list of variants | SIM-owned pathing/vision traits (see 1e). `#[serde(default)]` — omitted = `[]`. |
| `on_death` | `Option<OnDeathEffect>` | `Some(…)` | What the piece fans when DESTROYED (GTW-547; see 1e). `#[serde(default)]` — omitted = `None`. |

Kind variants are **struct variants**, so RON uses the single-paren named-field
form: `Slab(hp: 120, …)` — never the double-paren `Slab((…))` tuple form.

### 1c. `sim_kind:` — the four structural kinds

`TerrainSimKind` (`crates/gdtf_battle_sim/src/terrain/def/kind.rs`) has FOUR
variants. There is **no `Floor` kind** (a theme nominates a def as its
`default_floor` instead — see 1f) and **no `Scatter` kind** (loose debris is a
low `Cover`):

| `sim_kind:` variant | Destructible | Has height band | Extra |
|---------------------|--------------|-----------------|-------|
| `Wall(…)` | Yes | Yes | Fills the cell; nothing flies over it within a storey |
| `Cover(…)` | Yes | Yes | A round clears it by flying higher than its band |
| `Slab(…)` | Yes | **No** | Spans a z-boundary; stood on / under, never flown over |
| `Emplacement(…)` | Yes | Yes | A mounted-weapon position a ganger ENTERS to operate (GTW-543) |

`Wall` / `Cover` / `Emplacement` fields:

| Sub-field | Rust type | RON form | Notes |
|-----------|-----------|----------|-------|
| `hp` | `CoverHp` | bare integer | Full structural HP the piece seeds to |
| `armor_protection` | `ArmorProtection` (`i32`) | bare integer | Damage soak — same armor model as a ganger |
| `armor_hardness` | `ArmorHardness` (`i32`) | bare integer | Penetration this piece shrugs off |
| `height_band` | `HeightBand` | `Low` \| `Mid` \| `High` | Clearance band; a round must fly strictly higher to clear (`docs/combat/resolution.md` §3) |

`Emplacement` additionally carries:

| Sub-field | Rust type | RON form | Notes |
|-----------|-----------|----------|-------|
| `mounted_weapon` | `WeaponName` | bare string | The bolted-down gun's weapon registry key (a `.weapon.ron` file stem) |

`Slab` uses the distinct `SlabHp` pool and NO band (the march stops on an
intact slab regardless of the shot's band, `resolution.md` §2 — do not author a
`height_band:` on a slab):

| Sub-field | Rust type | RON form | Notes |
|-----------|-----------|----------|-------|
| `hp` | `SlabHp` | bare integer | Full structural HP (distinct pool from `CoverHp`) |
| `armor_protection` | `ArmorProtection` (`i32`) | bare integer | Damage soak |
| `armor_hardness` | `ArmorHardness` (`i32`) | bare integer | Penetration shrugged off |

### 1d. `presenter_kind:` — the presentation half

`TerrainPresenterKind` (same file as `TerrainSimKind`) mirrors the sim kinds
and carries ONLY presentation hooks; by the one-way sim→presenter dependency
the presenter reads this half and never the sim half. Every variant carries a
`graphic_name:` (`TerrainGraphicKey`, a bare string) — the graphic ROLE key the
presenter resolves via `TileRoles` to an atlas tile; the sim never touches
atlas indices. Only `Slab` adds an optional footfall:

| `presenter_kind:` variant | Fields |
|---------------------------|--------|
| `Wall(…)` | `graphic_name` |
| `Cover(…)` | `graphic_name` |
| `Slab(…)` | `graphic_name`, `footfall: Option<FootfallSound>` (`Some("footfall_metal")` / `None`) |
| `Emplacement(…)` | `graphic_name` |

Author the SAME structural kind on both halves (a `Slab` sim kind takes a
`Slab` presenter kind).

### 1e. `tags:` and `on_death:` — optional sim behavior

`tags:` lists `TerrainTag` variants — the closed, SIM-owned trait set that
drives pathing / vision (never authored on `presenter_kind`):

| Variant | Meaning |
|---------|---------|
| `Openable` | A door / hatch — passable when open, blocking when closed |
| `BlocksVision` | Stops line-of-sight / field-of-view |
| `BlocksPathfinding` | The cell is impassable |
| `Indestructible` | HP depletion can never destroy it |

Kind defaults already cover the common cases (a `Wall` / `Emplacement` blocks
path + vision by kind), so most defs author `tags: []`; solid walls in the
shipped content tag `[BlocksVision, BlocksPathfinding]` explicitly.

**Forward note (GTW-587, post-epic):** per-def blocking-OVERRIDE knobs (a def
opting out of its kind's blocking defaults) are planned but NOT built — today
the kind defaults + the additive tags above are the whole authored blocking
surface; do not author override fields.

`on_death:` names the `OnDeathEffect` a DESTRUCTIBLE piece fans when smashed
(GTW-547): `Explode(hit_type: …, damage: …, damage_type: …)` or
`LeaveField(field: "<field key>")`. Example from the shipped content
(`assets/content/terrain/sump_waste/waste_drum.terrain_def.ron`):

```ron
on_death: Some(LeaveField(field: "toxic_waste_pool")),
```

### 1f. The theme file — `<theme>.terrain_theme.ron`

Each theme folder carries ONE theme file, deserializing into `UuidThemeDef`
(`crates/gdtf_battle_sim/src/level/theme_def/definition.rs`):

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `key` | `ThemeUuid` | UUID string | The theme's stable key (prefabs / `skirmish.ron` reference it). |
| `display_name` | `ThemeDisplayName` | bare string | Human label (the editor's theme picker). |
| `default_floor` | `TerrainUuid` | UUID string | The terrain def a generated level fills empty ground with — a floor is just a def the theme nominates (authored as a walkable `Slab`; the move-cost-through-terrain seam is future work, see [terrain-content-migration.md](terrain-content-migration.md)). |
| `terrain` | `Vec<TerrainUuid>` | list of UUID strings | The theme's palette — the defs a generated level / the editor draws from. |

### 1g. Worked example — a new cover piece

Create `rusted_barrels.terrain_def.ron` under `assets/content/terrain/underhive/`:

```ron
// Rusted barrels — stacked low cover, underhive theme.
// PAYLOAD-KEYED: the `key:` UUID below is the identity; the filename is a courtesy.
(
    key: "3d3f9b52-6f6e-4b6a-9a3e-2f8c1d7e4a01",   // freshly minted v4 UUID
    display_name: "Rusted Barrels",
    sim_kind: Cover(
        hp:               18,    // fragile stacked drums
        armor_protection: 1,     // thin corroded steel
        armor_hardness:   0,     // no punch resistance
        height_band:      Low,   // LOW cover; a MID/HIGH round clears it
    ),
    presenter_kind: Cover(
        graphic_name: "cover",   // TileRoles role key -> atlas tile
    ),
    tags: [],                    // plain destructible cover; no extra traits
)
```

Then add its UUID to the theme palette in
`assets/content/terrain/underhive/underhive.terrain_theme.ron`:

```ron
    terrain: [
        // …existing UUIDs…
        "3d3f9b52-6f6e-4b6a-9a3e-2f8c1d7e4a01",  // rusted_barrels
    ],
```

That is the whole authoring loop — the folder load picks the file up at the
next launch (or live, via hot-reload — Part 3). The content editor
(`cargo run -p gdtf_content_editor_bin`) writes the same files to the same folder
(`crates/gdtf_content_editor/src/terrain_form/save.rs` /
`theme_form/save.rs`).

---

## Part 2 — How to extend the terrain model

This section is for engineers adding a new sim kind or a new field to the
model.

### Step 1 — Add a new variant to `TerrainSimKind` (new kind only)

File: `crates/gdtf_battle_sim/src/terrain/def/kind.rs`

Add a STRUCT variant (named fields — keeps the single-paren RON form) reusing
existing newtypes (`CoverHp`, `SlabHp`, `ArmorProtection`, `ArmorHardness`,
`HeightBand`, …) — no bare types. Add the mirroring `TerrainPresenterKind`
variant (graphic key only, unless the kind is walked on). The `Emplacement`
kind (GTW-543) is the template for a stateful kind.

Both payload-carrying kinds project onto the CANONICAL fieldless discriminant
`TerrainPieceKind` (GTW-574 —
`crates/gdtf_battle_sim/src/terrain/entity/components.rs`:
`Wall` / `Cover` / `Slab` / `Emplacement`, the queryable kind tag on every
terrain entity). Its exhaustive `kind()` projections and `TerrainPieceKind::ALL`
inventory compiler-tie every payload-free kind-identity decision (the editor
pick list, the occupancy bridge, the procgen classifier, the blocking
defaults) to the new variant — so a new kind is a COMPILE ERROR at each
decision point until you grow the discriminant, its projections, and the
`ALL` array alongside the two authored halves.

### Step 2 — Add a new field to an existing kind (or to `TerrainDef`)

Add the field + its newtype in `kind.rs` / `definition.rs`. If optional /
backwards-compatible, add `#[serde(default)]` (the `tags` / `on_death`
precedent — every shipped def stays parseable). If required, update every
`.terrain_def.ron` under `assets/content/terrain/`.

### Step 3 — Thread through the sim

Terrain defs are consumed at battle setup
(`crates/gdtf_battle_sim/src/lifecycle/situation/` — cover/slab UUID
resolution) and by the pathfinder / shot-march systems:

- Move / blocking: `crates/gdtf_battle_sim/src/perception/pathfinder/`.
- Vision / LOS: `crates/gdtf_battle_sim/src/perception/visibility/` and
  `crates/gdtf_battle_sim/src/shot_pipeline/march/`.

### Step 4 — Update tests and fixtures

The def round-trip tests live beside the types
(`crates/gdtf_battle_sim/src/terrain/def/test/`,
`crates/gdtf_battle_sim/src/level/theme_def/test/`); the shipped-content
integration test is `crates/gdtf_app/tests/migrated_terrain_content.rs`. A new
required field breaks their inline RON — add the field or `#[serde(default)]`.

### Step 5 — Update authoring docs

Add the new kind/field to the tables and a commented `.ron` example in this
guide. Run `cargo doc --workspace --no-deps` and confirm intra-doc links are
clean.

---

## Part 3 — Hot-reload

Terrain hot-reload rides the generic content-family seam (GTW-570): editing
any `*.terrain_def.ron` / `*.terrain_theme.ron` under
`assets/content/terrain/` while the game is running fires the generic
`redrive_content_family::<TerrainDefsFamily>` (resp. `::<ThemeDefsFamily>`)
system in `crates/gdtf_assets/src/family/systems.rs`, which rebuilds the whole
registry from the persistent `ContentFolderHandle` and overwrites it via
`ResMut`, marking it changed. An `info!` line
(``hot-reload: rebuilt TerrainDefRegistry from `content/terrain` ``) names the
reload. The next battle setup resolves against the edited defs without a
restart.

---

## Part 4 — Loader and key resolution

The terrain and theme loaders are instances of the ONE generic content-family
loader (GTW-570):

- Family markers: `TerrainDefsFamily` / `ThemeDefsFamily` in
  `crates/gdtf_content_families/src/terrain_defs.rs` / `theme_defs.rs` — each
  declares its folder (`content/terrain` for BOTH), its dedicated compound
  extension (`terrain_def.ron` / `terrain_theme.ron`), and its payload-keyed
  `insert_member` (keys by the def's own `key:` UUID; the file stem is unused).
- Generic systems: `crates/gdtf_assets/src/family/systems.rs` (Startup
  kick-off → gated resolve → live redrive).
- Registration site: `crates/gdtf_app/src/states/load/plugin.rs` — one
  `app.register_content_family::<TerrainDefsFamily>()` +
  `::<ThemeDefsFamily>()` call each (the
  `ContentFamilyAppExt` extension, `crates/gdtf_assets/src/family/ext.rs`).

The load flow, per family:

1. `Startup`: `load_folder("content/terrain")` recursively; the persistent
   `ContentFolderHandle<F>` is stored (it also feeds the redrive + the file
   watcher).
2. The resolve gates on `RecursiveDependencyLoadState::Loaded`, then walks the
   folder: each member is filtered by asset `TypeId` first — the terrain walk
   skips the theme members and vice-versa (the two families share the mixed
   tree) — and folded into the registry via the family's `insert_member`,
   keyed by the def's own UUID.
3. The resolved registries are inserted exactly once:
   `TerrainDefRegistry` (`TerrainUuid` → `TerrainDef`,
   `crates/gdtf_battle_sim/src/terrain/def/registry.rs`) and
   `UuidThemeRegistry` (`ThemeUuid` → `UuidThemeDef`,
   `crates/gdtf_battle_sim/src/level/theme_def/registry.rs`). Both are named
   newtypes over the foundation `Registry<K, V>` map
   (`crates/gdtf_battle_sim/src/foundation/registry/map.rs`, GTW-567).
4. On a genuine folder `Failed` the loader `warn!`s and inserts an EMPTY
   registry so `Load` always exits with one present (ADR-0003 safety-net); a
   battle then fails closed on a missing UUID rather than crashing.

Key Rust types:

- `TerrainDef`, `TerrainDisplayName` —
  `crates/gdtf_battle_sim/src/terrain/def/definition.rs`
- `TerrainUuid` — `crates/gdtf_battle_sim/src/terrain/def/uuid.rs`
- `TerrainSimKind`, `TerrainPresenterKind`, `TerrainTag` —
  `crates/gdtf_battle_sim/src/terrain/def/kind.rs`
- `TerrainDefRegistry` — `crates/gdtf_battle_sim/src/terrain/def/registry.rs`
- `UuidThemeDef`, `ThemeDisplayName` —
  `crates/gdtf_battle_sim/src/level/theme_def/definition.rs`
- `ThemeUuid` — `crates/gdtf_battle_sim/src/level/theme_def/uuid.rs`
- `UuidThemeRegistry` — `crates/gdtf_battle_sim/src/level/theme_def/registry.rs`

Supporting newtypes from other modules:

- `CoverHp`, `HeightBand` — `crates/gdtf_battle_sim/src/terrain/cover/types.rs`
- `SlabHp` — `crates/gdtf_battle_sim/src/terrain/slab/types.rs`
- `ArmorProtection`, `ArmorHardness` —
  `crates/gdtf_battle_sim/src/equipment/armor/stats.rs`
- `TerrainGraphicKey`, `FootfallSound` —
  `crates/gdtf_battle_sim/src/terrain/piece/components.rs`
- `WeaponName` — `crates/gdtf_battle_sim/src/equipment/weapon/components/handling.rs`
- `OnDeathEffect` — `crates/gdtf_battle_sim/src/effects/on_death/effect.rs`

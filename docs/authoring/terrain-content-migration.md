# Terrain / theme / prefab content migration — reconciliation ruling (GTW-490)

This note is the durable in-repo record (C1-RECORD) of the reconciliation ruling
applied when migrating the shipped content to the NEW per-theme, UUID-keyed
terrain / theme / prefab model. It lives in `docs/` (NOT beside the migrated
`.ron` content under `assets/terrain/` or `assets/maps/`, because those are the
recursively-loaded asset folders — a stray non-asset file there fails the
`load_folder` walk).

The migrated content is shipped under:

- `assets/terrain/<theme>/<tile>.terrain_def.ron` — the GTW-487 UUID-keyed
  `TerrainDef` files.
- `assets/terrain/<theme>/<theme>.terrain_theme.ron` — the GTW-487 `UuidThemeDef`
  files.
- `assets/maps/<theme>/<size>/<name>.prefab_v2.ron` — the GTW-489
  `PrefabSpecV2` files.

## The dual-catalog conflict (industrial_hive)

Before this migration two divergent catalogs described the *industrial_hive*
terrain:

- The **flat `TerrainRegistry`** (`assets/content/terrain/*.terrain.ron`, keyed by
  file stem) — the catalog the LIVE battle path, the prefabs, and `skirmish.ron`
  actually reference. **8 pieces:** `barricade`, `bulkhead_wall`, `debris_pile`,
  `deck_floor`, `deck_slab`, `gantry_slab`, `heavy_bulkhead`, `supply_crate`.
- The **parallel `ThemeTileCatalog`** (`assets/content/themes/industrial_hive.theme.ron`,
  the editor-palette / old theme loader) — keyed `industrial_hive`'s **4 tiles:**
  `deck_plating`, `bulkhead_wall`, `supply_crate`, `deck_slab`. It lacks
  `barricade` / `heavy_bulkhead` / `debris_pile` / `gantry_slab`, and its
  `deck_plating` is not `deck_floor`.

## Ruling (applied)

1. **The flat catalog is the SOURCE OF TRUTH.** It is what the live battle path,
   the prefabs, and `skirmish.ron` reference. The unified per-theme
   industrial_hive terrain set is the **flat 8 pieces** — each migrated to a
   `TerrainDef` with a generated, stable `TerrainUuid`. No tile is silently
   dropped.
2. **`deck_plating` -> `deck_floor`.** The theme-catalog tile key `deck_plating`
   maps onto the flat `deck_floor` piece by display intent (both are the theme's
   default walkable plating). The theme's `default_floor` UUID is the migrated
   `deck_floor` def's UUID.
3. **`atlas_index` is INTENTIONALLY DROPPED.** The old `ThemeTileCatalog`
   `CatalogTile.atlas_index` does not survive the migration. The presenter
   resolves the atlas index via `TileRoles` keyed by the def's
   `presenter_kind.graphic_name` (the old flat file's `graphic` `TileRoles` key),
   NEVER a per-def atlas index.

## Kind mapping (the new model)

The new `TerrainSimKind` is `Wall` / `Cover` / `Slab` ONLY (no `Floor`, no
`Scatter`), and `TerrainDef` carries NO `MoveCost` field in this slice.

- **Retired `Floor` kind (sub-decision).** A floor is just the terrain a theme
  nominates as its `default_floor`; the floor's move cost will (in a later T07+
  consumer slice) ride that referenced terrain. Because there is no `MoveCost`
  field on `TerrainDef` yet, the migrated floor defs (`deck_floor`,
  `rockcrete_floor`, `sludge_floor`) are authored as a `Slab` sim_kind — the
  GTW-487 `deck.terrain_def.ron` walkable-default-floor fixture precedent. The
  flat / catalog floor authored ONLY a move cost (no HP/armor), so each migrated
  floor-as-slab's structural toughness is COPIED from a sibling structural slab in
  the SAME theme (industrial_hive `deck_floor` <- `deck_slab` 120/4/2; underhive
  `rockcrete_floor` <- `gantry_slab` 35/4/2; sump_waste `sludge_floor` <-
  `gantry_slab` 35/4/2). The real authored move-cost value (4 / 5 / 6) is carried
  FORWARD as a documented note in each floor def file until the T07 move-cost seam
  lands — it is recorded, not silently dropped.
- **`Scatter` folds into `Cover`** (the new model has no `Scatter` variant):
  `barricade` (already flat-Cover), `debris_pile`, `scrap_barricade`,
  `rubble_pile` are `Cover`.
- All HP / armor / band MAGNITUDES are COPIED verbatim from the flat / catalog
  files (behavior-preserving — no re-tune).
- **Sim-owned tags.** Solid walls (`bulkhead_wall`, `heavy_bulkhead`,
  `tunnel_wall`, `corroded_bulkhead`) are tagged `[BlocksVision,
  BlocksPathfinding]` sim-side (consumption is GTW-482); tags are NEVER on
  `presenter_kind`.

## The other two themes (underhive, sump_waste)

These have no flat counterpart (only their own `ThemeTileCatalog` tiles). They
migrate their OWN tiles to per-theme `TerrainDef`s under `terrain/<theme>/`,
dropping `atlas_index` the same way (mapping each old index to its matching
`TileRoles` graphic key), folding `Scatter` into `Cover`, and generating stable
UUIDs. Their `default_floor` is the migrated floor def of that theme.

## Prefab migration sub-decisions

- **`role` omitted (serde-default `Fill`).** The migrated v2 prefabs
  (`maps/industrial_hive/{3x3,12x12}/*.prefab_v2.ron`) OMIT the `role` field, so it
  deserializes as `SpawnRole::Fill` (the GTW-490 recipe step 3 + C3 "role defaults
  applied"). The legacy `Player` / `Enemy` role intent is not yet meaningful — the
  v2 assembler (T07b) does not consume these prefabs and will re-key role then.
- **`default_floor` + `edge_openings` dropped.** The legacy per-prefab
  `default_floor` is a theme concern now (the theme's `default_floor`); the legacy
  `edge_openings` has no v2 field (connectivity is by-construction in the v2
  assembler). The v2 `placements` list maps only the old `walls` + `scatter` stem
  refs to migrated `TerrainUuid`s.

## skirmish.ron sub-decision (C5)

`skirmish.ron` AUTHORS BOTH: the legacy `theme: IndustrialHive` (UNCHANGED — keeps
the live battle path working, C7) AND a new ADDITIVE `theme_uuid: Some("…")` that
names the migrated IndustrialHive `ThemeUuid`. A new `Option<ThemeUuid>`
`#[serde(default)]` field was added to `Situation` (it switches NO consumer; the
live path still reads the `theme` enum). C5's situation-loader test reads
`theme_uuid` off the resolved situation and asserts it resolves in
`UuidThemeRegistry`. Replacing the `theme` enum outright would be a consumer switch
(T07+) and would break the live path — out of scope here.

## Stable authored UUID scheme

All UUIDs are authored constants (`Uuid::from_u128`-style), used consistently
across terrain defs, theme files, prefab placements, and `skirmish.ron`. A terrain
UUID in a prefab placement MUST match the terrain def's key; the theme UUID in a
prefab / skirmish MUST match the theme file's key.

| concept                          | UUID                                   |
| -------------------------------- | -------------------------------------- |
| theme industrial_hive            | `00000000-0000-0000-0000-01840a900001` |
| theme underhive                  | `00000000-0000-0000-0000-01840a900002` |
| theme sump_waste                 | `00000000-0000-0000-0000-01840a900003` |
| ih barricade                     | `00000000-0000-0000-0000-01840a910001` |
| ih bulkhead_wall                 | `00000000-0000-0000-0000-01840a910002` |
| ih debris_pile                   | `00000000-0000-0000-0000-01840a910003` |
| ih deck_floor (default floor)    | `00000000-0000-0000-0000-01840a910004` |
| ih deck_slab                     | `00000000-0000-0000-0000-01840a910005` |
| ih gantry_slab                   | `00000000-0000-0000-0000-01840a910006` |
| ih heavy_bulkhead                | `00000000-0000-0000-0000-01840a910007` |
| ih supply_crate                  | `00000000-0000-0000-0000-01840a910008` |
| uh rockcrete_floor (def floor)   | `00000000-0000-0000-0000-01840a920001` |
| uh tunnel_wall                   | `00000000-0000-0000-0000-01840a920002` |
| uh scrap_barricade               | `00000000-0000-0000-0000-01840a920003` |
| uh rubble_pile                   | `00000000-0000-0000-0000-01840a920004` |
| sw sludge_floor (def floor)      | `00000000-0000-0000-0000-01840a930001` |
| sw corroded_bulkhead             | `00000000-0000-0000-0000-01840a930002` |
| sw gantry_slab                   | `00000000-0000-0000-0000-01840a930003` |
| sw waste_drum                    | `00000000-0000-0000-0000-01840a930004` |

## Scope boundary

ADDITIVE only. The OLD flat `content/terrain/*.terrain.ron`, the OLD
`content/themes/*.theme.ron`, the OLD `content/maps/**/*.prefab.ron`, and their
OLD loaders stay in place and LIVE — the live game still runs on the old path.
This migration only fills the NEW registries. Switching consumers to the new
model is T07+.

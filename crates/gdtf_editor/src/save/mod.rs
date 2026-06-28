//! The editor **SAVE-PREFAB path** (GTW-432): project the in-memory [`EditorMap`](crate::EditorMap) into the
//! canonical GTW-418 [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec) schema, serialize it to RON, and WRITE it to
//! `assets/content/maps/<theme>/<size>/<prefab_name>.prefab.ron`.
//!
//! ## The schema the saver writes (C1 / C2)
//!
//! The saver builds a real [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec) — the SAME type the GTW-418 folder loader
//! deserializes (NOT a parallel schema) — so a saved prefab round-trips through that loader
//! with no data loss (`load(save(grid)) == grid`). Every painted cell of the [`EditorMap`](crate::EditorMap)
//! (including multi-level cells) is routed into the matching prefab list by its editor tile
//! class + catalog kind:
//!
//! - a SLAB tile → a [`SlabSpawn`](gdtf_battle_sim::situation::SlabSpawn) in [`PrefabSpec::slabs`](gdtf_battle_sim::level::PrefabSpec::slabs),
//! - a LADDER tile → a [`VerticalLink`](gdtf_battle_sim::vertical::VerticalLink) in [`PrefabSpec::vertical_links`](gdtf_battle_sim::level::PrefabSpec::vertical_links) (a bidirectional
//!   ladder rising to the storey above — the only authored way a ganger changes storey),
//! - a WALL catalog tile → a [`CoverSpawn`](gdtf_battle_sim::situation::CoverSpawn) in [`PrefabSpec::walls`](gdtf_battle_sim::level::PrefabSpec::walls),
//! - a COVER / SCATTER catalog tile → a [`CoverSpawn`](gdtf_battle_sim::situation::CoverSpawn) in [`PrefabSpec::scatter`](gdtf_battle_sim::level::PrefabSpec::scatter),
//! - a FLOOR catalog tile → a [`FloorSpawn`](gdtf_battle_sim::situation::FloorSpawn) per-cell override in [`PrefabSpec::floors`](gdtf_battle_sim::level::PrefabSpec::floors).
//!
//! The prefab's [`theme`](gdtf_battle_sim::level::PrefabSpec::theme) + [`size`](gdtf_battle_sim::level::PrefabSpec::size) come from the
//! authoring [`MapEditorSession`](crate::MapEditorSession), and the [`spawn_role`](gdtf_battle_sim::level::PrefabSpec::spawn_role) defaults to
//! [`SpawnRole::Fill`](gdtf_battle_sim::level::SpawnRole::Fill) — the connective default (the editor has no spawn-role control yet; Fill
//! is the safe generic role the loader buckets a fragment under, the [`PrefabSpec::default`](gdtf_battle_sim::level::PrefabSpec::default)
//! choice). The [`default_floor`](gdtf_battle_sim::level::PrefabSpec::default_floor) comes from the session's resolved
//! default-floor key.
//!
//! ## `TileKey` → `TerrainName` (the catalog-vs-terrain vocabulary bridge)
//!
//! The [`EditorMap`](crate::EditorMap) stores a painted cell's catalog [`TileKey`](gdtf_battle_sim::level::TileKey), but the prefab schema names
//! its pieces by [`TerrainName`](gdtf_battle_sim::terrain::piece::TerrainName) (a terrain-file stem). By shipped convention the two share the
//! same string for placeable pieces (`bulkhead_wall`, `supply_crate`, `deck_slab`, …), so the
//! saver reinterprets a [`TileKey`](gdtf_battle_sim::level::TileKey)'s string as a [`TerrainName`](gdtf_battle_sim::terrain::piece::TerrainName) — the same identity the GTW-418
//! consumer later resolves against the `TerrainRegistry`. This is the documented chosen bridge
//! (the catalog has no terrain-name mapping table).
//!
//! ## Edge openings (C6, derived)
//!
//! The GTW-418 loader REJECTS a prefab with zero [`EdgeOpening`](gdtf_battle_sim::level::EdgeOpening)s (connectivity-by-construction),
//! so a saved prefab MUST author at least one or it would not survive the round-trip. The editor
//! has no edge-opening UI, so the saver DERIVES them: every walkable floor cell on a footprint
//! BOUNDARY edge (a default-floor cell or a per-cell floor override) is an edge opening the
//! inter-prefab seam can connect to. If the map authors no walkable boundary cell, the save is
//! REJECTED (a typed [`SavePrefabError`], surfaced via `error!` — never a panic), because such a
//! prefab cannot connect and the loader would exclude it anyway.
//!
//! ## Illegal-cell guard (C3)
//!
//! Before serializing, the saver re-checks EVERY painted cell against the GTW-430 shared
//! [`evaluate_placement`](crate::evaluate_placement) predicate; if any cell is an illegal placement the save is REJECTED
//! (a typed [`SavePrefabError::IllegalCell`]) and NOTHING is written — a saved prefab never
//! contains an illegal cell.
//!
//! ## Debug-only (C1)
//!
//! The WHOLE module is `#[cfg(debug_assertions)]`-gated at its `mod` site (the GTW-429 gang-save
//! precedent), so the filesystem write + the conversion + the press system are never compiled
//! into a release binary. Every fallible step is handled by LOGGING + early-return — never
//! `unwrap`/`expect`/`panic` (the no-panic contract + the workspace-denied lints).
//!
//! ## Module layout
//!
//! | Submodule    | Concern |
//! |--------------|---------|
//! | [`types`]    | Path consts, component markers ([`PrefabNameField`], [`SavePrefabButton`]), [`SavePrefabError`] |
//! | [`project`]  | Pure projection + serialization: [`editor_map_to_prefab`](project::editor_map_to_prefab), [`serialize_prefab`](project::serialize_prefab), [`prefab_save_path`](project::prefab_save_path) |
//! | [`systems`]  | Filesystem writer ([`write_prefab`](systems::write_prefab)) + Bevy systems ([`spawn_save_controls`](systems::spawn_save_controls), [`save_prefab_on_press`](systems::save_prefab_on_press)) |
//! | [`tests`]    | In-crate tests (4 contract-clause tests) |

mod project;
mod systems;
mod types;

#[cfg(test)]
mod tests;

// Re-export the `pub(crate)` surface consumed by `plugin.rs` (the only external caller).
// The submodules expose additional items internally via `pub(super)` / `pub(crate)` where
// needed; tests import from submodule paths directly.
pub(crate) use systems::{save_prefab_on_press, spawn_save_controls};

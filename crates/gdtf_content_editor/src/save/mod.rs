//! The editor **SAVE-PREFAB path** (GTW-432; swept onto the v2 UUID schema in GTW-495): project
//! the in-memory [`EditorMap`](crate::EditorMap) into the v2
//! [`PrefabSpecV2`](gdtf_battle_sim::level::PrefabSpecV2) schema, serialize it to RON, and WRITE
//! it to `assets/maps/<theme>/<size>/<prefab_name>.prefab_v2.ron`.
//!
//! ## The schema the saver writes (C2)
//!
//! The saver builds a real [`PrefabSpecV2`](gdtf_battle_sim::level::PrefabSpecV2) — the SAME type
//! the GTW-489 v2 folder loader deserializes — so a saved prefab round-trips through that loader
//! with no data loss (`load(save(grid)) == grid`). Every painted cell of the
//! [`EditorMap`](crate::EditorMap) (including multi-level cells) collapses into ONE
//! [`placements`](gdtf_battle_sim::level::PrefabSpecV2::placements) list of
//! [`TerrainPlacementEntry`](gdtf_battle_sim::level::TerrainPlacementEntry) — `(piece, at)` pairs
//! referencing the painted [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid) — because
//! the per-piece behaviour (wall / cover / slab) now lives in the referenced
//! [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef), not in split lists.
//!
//! The prefab's [`theme`](gdtf_battle_sim::level::PrefabSpecV2::theme) (a
//! [`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid)) + [`size`](gdtf_battle_sim::level::PrefabSpecV2::size)
//! come from the authoring [`MapEditorSession`](crate::MapEditorSession), and the
//! [`role`](gdtf_battle_sim::level::PrefabSpecV2::role) defaults to
//! [`SpawnRole::Fill`](gdtf_battle_sim::level::SpawnRole::Fill) — the connective default (the
//! editor has no spawn-role control yet).
//!
//! ## No authored openings (GTW-495)
//!
//! The v2 schema carries NO authored-opening field — inter-fragment connectivity is
//! by-construction in the v2 assembler (the 1-cell `default_floor` seam every placement reserves),
//! not authored per-prefab — so the saver derives NONE (the legacy opening-derivation + zero-opening
//! rejection are GONE; the underlying machinery was removed in GTW-497). A zero-placement prefab is
//! a valid v2 prefab (the infallible `Prefab2::new`).
//!
//! ## Illegal-cell guard (C3)
//!
//! Before serializing, the saver re-checks EVERY painted cell against the GTW-430 shared
//! [`evaluate_placement`](crate::evaluate_placement) predicate; if any cell is an illegal
//! placement the save is REJECTED (a typed [`SavePrefabError::IllegalCell`]) and NOTHING is
//! written — a saved prefab never contains an illegal cell.
//!
//! ## Debug-only (C1)
//!
//! The WHOLE module is `#[cfg(debug_assertions)]`-gated at its `mod` site (the GTW-429 gang-save
//! precedent), so the filesystem write + the conversion + the press system are never compiled into
//! a release binary. Every fallible step is handled by LOGGING + early-return — never
//! `unwrap`/`expect`/`panic`.
//!
//! ## GTW-512: the egui swap — projection kept, the `bevy_ui` controls deferred to C4
//!
//! The egui migration (GTW-512 C1) keeps the pure PROJECTION + serialization (the [`types`] consts /
//! markers / errors + [`project`]'s `EditorMap` → `PrefabSpecV2` projection + RON serialize + path
//! resolution) — the contract the C4 child builds on — and DROPS the `bevy_ui` save controls: the
//! `systems` module (the filesystem writer + the prefab-name field / "Save prefab" button spawn +
//! the press trigger) is GONE from the module tree. The egui save controls + the fs-write press
//! re-point are the C4 child (GTW-515).
//!
//! ## Module layout (post-egui-swap)
//!
//! | Submodule    | Concern |
//! |--------------|---------|
//! | [`types`]    | Path consts, component markers ([`PrefabNameField`](types::PrefabNameField), [`SavePrefabButton`](types::SavePrefabButton)), [`SavePrefabError`](types::SavePrefabError), theme/size dir helpers |
//! | [`project`]  | Pure projection + serialization: [`editor_map_to_prefab`](project::editor_map_to_prefab), [`serialize_prefab`](project::serialize_prefab), [`prefab_save_path`](project::prefab_save_path), [`sanitize_name`](project::sanitize_name) |
//! | [`tests`]    | In-crate tests (the contract-clause tests) |

mod project;
mod types;

#[cfg(test)]
mod tests;

// The pure projection + serialization + path-resolution surface, re-exported for the crate root
// (`lib.rs`) so the C4 save-control re-point + the in-crate save tests reach it (GTW-512). The
// `bevy_ui` save controls themselves are the C4 child.
pub use project::{editor_map_to_prefab, prefab_save_path, sanitize_name, serialize_prefab};
pub use types::SavePrefabError;

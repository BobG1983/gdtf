//! The TERRAIN authoring mode of the Workbench editor (GTW-474) — the MODEL + SAVE for authoring a
//! terrain definition (`*.terrain_def.ron`) on the UUID model.
//!
//! The form captures the full [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) shape:
//! `display_name` + `sim_kind` (Wall / Cover / Slab ONLY) + the per-kind sim stats
//! (`CoverHp` / `SlabHp` / `ArmorProtection` / `ArmorHardness` / `HeightBand`) +
//! `presenter_kind.graphic_name` + `footfall` (Slab only) + the multi-select `tags`. The UUID is
//! editor-generated on the first save.
//!
//! SAVE projects the in-progress [`TerrainDraft`](types::TerrainDraft) into a real `TerrainDef`,
//! serializes it to RON, and writes it to `assets/content/terrain/<theme>/<name>.terrain_def.ron` so the
//! GTW-487 terrain loader resolves it into the
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) (hot-reload).
//!
//! ## GTW-512: the egui swap — MODEL kept, the `bevy_ui` form deferred to C2
//!
//! The egui migration (GTW-512 C1) keeps the MODEL + SAVE (the [`types`] draft / pick enums /
//! markers / input newtypes + [`save`]'s pure projection + serialization + the debug-only fs write)
//! — the contract every later child builds on — and DROPS the `bevy_ui` form: the GTW-474 `spawn`
//! layout + the `systems` commit / reflow / RON-preview drive systems are GONE from the module tree.
//! The TERRAIN egui form (the full re-point of the drive systems onto egui widgets) is the C2 child
//! (GTW-513); the right [`SidePanel`](bevy_egui::egui::SidePanel) shows a stub until then.
//!
//! ## Module layout (post-egui-swap)
//!
//! | Submodule | Concern |
//! |-----------|---------|
//! | [`types`] | The [`TerrainDraft`](types::TerrainDraft), the kind / footfall pick enums, the [`offered_graphic_roles`](types::offered_graphic_roles) pick over the presenter's `TileRole` vocabulary (GTW-566 C5), the numeric-input newtypes, and [`SaveTerrainError`](types::SaveTerrainError) |
//! | [`save`]  | The pure projection + serialization + the debug-only fs write |
//! | [`tests`] | In-crate tests (the C2 footfall gate + the C3 round-trip + the GTW-566 picker derivation) |

mod save;
mod types;

#[cfg(test)]
mod tests;

pub use save::{draft_to_terrain_def, serialize_terrain_def};
// The debug-only fs write (projects + serializes + writes the `.terrain_def.ron`) — kept for the C2
// child's egui save-press re-point (GTW-512). Re-exporting both the root-parameterized core
// (`write_terrain_in`) and the production wrapper (`write_terrain`) keeps the test isolation path
// reachable without exposing the workspace root const to callers.
#[cfg(debug_assertions)]
pub use save::{write_terrain, write_terrain_in};
pub use types::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainError, TerrainDraft, TerrainKindChoice,
    offered_graphic_roles,
};

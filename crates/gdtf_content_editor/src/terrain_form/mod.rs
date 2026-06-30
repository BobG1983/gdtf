//! The TERRAIN authoring mode of the Workbench editor (GTW-474) — author a terrain definition
//! (`*.terrain_def.ron`) in-editor on the UUID model.
//!
//! The form captures the full [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) shape:
//! `display_name` + `sim_kind` (Wall / Cover / Slab ONLY) + the per-kind sim stats
//! (`CoverHp` / `SlabHp` / `ArmorProtection` / `ArmorHardness` / `HeightBand`) +
//! `presenter_kind.graphic_name` (a per-`TileRoles`-role picker, NOT a raw atlas index) +
//! `footfall` (offered ONLY when the kind is Slab — C2 gate) + the multi-select `tags`
//! (`Openable` / `BlocksVision` / `BlocksPathfinding` / `Indestructible`). The UUID is
//! editor-generated
//! on the first save (shown read-only).
//!
//! SAVE projects the in-progress [`TerrainDraft`](types::TerrainDraft) into a real `TerrainDef`,
//! serializes it to RON, and writes it to `assets/terrain/<theme>/<name>.terrain_def.ron` so the
//! GTW-487 terrain loader resolves it into the
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) (hot-reload).
//!
//! ## Module layout
//!
//! | Submodule    | Concern |
//! |--------------|---------|
//! | [`types`]    | The [`TerrainDraft`](types::TerrainDraft), the kind / graphic / footfall pick enums, the field markers, the numeric-input newtypes, and [`SaveTerrainError`](types::SaveTerrainError) |
//! | [`spawn`]    | `OnEnter(Editing)` layout: the form's widgets into the three regions' TERRAIN containers |
//! | [`systems`]  | The `Update` drive systems: commit capture, the kind reflow + footfall gate, the live RON preview, and the save press |
//! | [`save`]     | The pure projection + serialization + the debug-only fs write |
//! | [`tests`]    | In-crate tests (the C2 footfall gate + the C3 round-trip) |

mod save;
mod spawn;
mod systems;
mod types;

#[cfg(test)]
mod tests;

pub use save::{draft_to_terrain_def, serialize_terrain_def};
pub(crate) use spawn::spawn_terrain_form;
#[cfg(debug_assertions)]
pub(crate) use systems::save_terrain_on_press;
pub(crate) use systems::{
    apply_terrain_band, apply_terrain_footfall, apply_terrain_kind, commit_terrain_armor,
    commit_terrain_hp, commit_terrain_name, gate_footfall_field, reflow_band_field,
    refresh_ron_preview, select_terrain_graphic, toggle_terrain_tag,
};
pub use types::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainError, TerrainDraft, TerrainFootfallPicker,
    TerrainGraphicChoice, TerrainKindChoice, TerrainKindTabs,
};

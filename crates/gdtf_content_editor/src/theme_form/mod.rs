//! The THEME authoring mode of the Workbench editor (GTW-475) — author a
//! [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) in-editor on the UUID model.
//!
//! The form captures the full theme shape BY REFERENCE: a `display_name`, a multi-selected
//! `terrain: Vec<TerrainUuid>` chosen from the loaded
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) library (C2), and one
//! of those terrain as the `default_floor` (Slab-kind preferred — C6). The theme stores
//! references ONLY — never inlined stats (C3); a read-only RESOLVED-STATS readout proves it by
//! resolving the selected default-floor UUID against the registry. The [`ThemeUuid`] is
//! editor-minted (a fresh one for a New theme, or loaded from the
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) when editing — C4) and shown
//! read-only.
//!
//! SAVE projects the in-progress [`ThemeDraft`](types::ThemeDraft) into a real `UuidThemeDef`,
//! serializes it to RON, and writes it to `assets/terrain/<slug>/<slug>.terrain_theme.ron` so the
//! GTW-487 theme loader (`resolve_theme_defs`) resolves it into the `UuidThemeRegistry`
//! (hot-reload). The save honors the C6 default-floor-must-be-own-terrain rule.
//!
//! ## Module layout
//!
//! | Submodule    | Concern |
//! |--------------|---------|
//! | [`types`]    | The [`ThemeDraft`](types::ThemeDraft), the field / identity markers, and [`SaveThemeError`](types::SaveThemeError) |
//! | [`spawn`]    | `OnEnter(Editing)` layout: the form's widgets into the four regions' THEME containers + the terrain-library row builder |
//! | [`systems`]  | The `Update` drive systems: name commit, the terrain multi-select, the library + default-floor sync, and the C4 load / new-theme |
//! | [`render`]   | The read-only render systems: the C3 resolved-stats readout + HP bar, the read-only KEY text, the live RON preview, and the debug-only save press |
//! | [`save`]     | The pure projection + serialization + the C6 validation + the debug-only fs write |
//! | [`tests`]    | In-crate tests (the C7 projection + round-trip, the C6 default-floor rule, the C3 resolution) |

mod render;
mod save;
mod spawn;
mod systems;
mod types;

#[cfg(test)]
mod tests;

#[cfg(debug_assertions)]
pub(crate) use render::save_theme_on_press;
pub(crate) use render::{
    refresh_resolved_stats, refresh_theme_key_text, refresh_theme_ron_preview,
};
pub use save::{draft_to_theme_def, serialize_theme_def, validate_for_save};
pub(crate) use spawn::spawn_theme_form;
pub(crate) use systems::{
    apply_default_floor, commit_theme_name, load_theme_into_form, reset_theme_form_on_new,
    sync_default_floor_options, sync_theme_library, toggle_theme_terrain,
};
pub use types::{
    SaveThemeError, ThemeDefaultFloorPicker, ThemeDraft, ThemeResolvedStatsText, ThemeTerrainRow,
};

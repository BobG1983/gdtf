//! The THEME authoring mode of the Workbench editor (GTW-475) — the MODEL + SAVE for authoring a
//! [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) on the UUID model.
//!
//! The form captures the full theme shape BY REFERENCE: a `display_name`, a multi-selected
//! `terrain: Vec<TerrainUuid>` chosen from the loaded
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) library, and one of
//! those terrain as the `default_floor`. The theme stores references ONLY — never inlined stats.
//!
//! SAVE projects the in-progress [`ThemeDraft`](types::ThemeDraft) into a real `UuidThemeDef`,
//! serializes it to RON, and writes it to `assets/terrain/<slug>/<slug>.terrain_theme.ron` so the
//! GTW-487 theme loader resolves it into the
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) (hot-reload). The save honors the
//! default-floor-must-be-own-terrain rule.
//!
//! ## GTW-512: the egui swap — MODEL kept, the `bevy_ui` form deferred to C3
//!
//! The egui migration (GTW-512 C1) keeps the MODEL + SAVE (the [`types`] draft / markers + [`save`]'s
//! pure projection + serialization + validation + the debug-only fs write) — the contract the C3
//! child builds on — and DROPS the `bevy_ui` form: the GTW-475 `spawn` layout, the `systems` drive,
//! and the `render` read-only systems are GONE from the module tree. The THEME egui form (the full
//! re-point of the drive + the resolved-stats readout onto egui widgets) is the C3 child (GTW-514);
//! the right [`SidePanel`](bevy_egui::egui::SidePanel) shows a stub until then.
//!
//! ## Module layout (post-egui-swap)
//!
//! | Submodule | Concern |
//! |-----------|---------|
//! | [`types`]   | The [`ThemeDraft`](types::ThemeDraft), the field / identity markers, and [`SaveThemeError`](types::SaveThemeError) |
//! | [`save`]    | The pure projection + serialization + the validation + the debug-only fs write |
//! | [`resolve`] | Pure resolution helpers ([`resolved_stats`](resolve::resolved_stats) / [`floor_candidates`](resolve::floor_candidates) / `sim_kind_label`) the C3 form + the tests reuse |
//! | [`tests`]   | In-crate tests (the projection + round-trip, the default-floor rule, the resolution) |

mod resolve;
mod save;
mod types;

#[cfg(test)]
mod tests;

pub use resolve::{floor_candidates, resolved_stats};
// The debug-only fs write (validates + projects + serializes + writes the `.terrain_theme.ron`) —
// kept for the C3 child's egui save-press re-point (GTW-512). Re-exporting it keeps its path helpers
// reachable.
#[cfg(debug_assertions)]
pub use save::write_theme;
pub use save::{draft_to_theme_def, serialize_theme_def, validate_for_save};
pub use types::{SaveThemeError, ThemeDraft};

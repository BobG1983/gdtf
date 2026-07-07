//! The egui THEME-mode authoring form (GTW-514 C3) — the real theme form that replaces the
//! C1 stub in the right panel + left stats + central (RON-preview) regions of the egui shell.
//!
//! This module is the egui DRAW + the debug-only save press for the THEME mode. It REUSES the
//! [`theme_form`](crate::theme_form) model + save VERBATIM (C3.3): every control reads / writes
//! the state-scoped [`ThemeDraft`](crate::theme_form::ThemeDraft) resource through its existing accessors / setters, and the
//! save button calls the existing [`write_theme`](crate::theme_form::write_theme) — round-tripping
//! the GTW-489 theme loader. The egui draw lives in
//! [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) (bevy-traps #8) via the caller
//! [`editor_egui_ui`](super::shell::editor_egui_ui); the controls mutate the draft in place
//! (no message round-trip — egui is immediate-mode), so the mutations are idempotent under the
//! multipass re-run (bevy-traps #8 fact (b)).
//!
//! ## Layout across the shell panels (GTW-530 — reworked emphasis)
//!
//! GTW-530 REWORKS the GTW-514 THEME layout so the terrain LIBRARY is the primary focus and the
//! `.terrain_theme.ron` preview is REMOVED entirely (unlike the TERRAIN tab's GTW-534 DEMOTION —
//! here the author never wants the RON, so the central space it held is reclaimed for the library):
//!
//! - CENTRAL primary panel — the [`terrain_library_panel`]: the terrain multi-select library, now
//!   the largest / most-prominent region. Each row renders `[sprite thumbnail] name [Kind]` — the
//!   sprite resolved via the SHARED [`sprite_thumb`](crate::egui_shell::sprite_thumb) helper
//!   (GTW-516) over the terrain's [`terrain_sprite_def`](crate::terrain_graphics::terrain_sprite_def)
//!   (GTW-665), NO hardcoded index. The check/uncheck multi-select (fail-closed default floor) is
//!   preserved.
//! - LEFT palette panel — the resolved-stats readout for the current default-floor terrain (or a
//!   placeholder when none is selected), reusing [`resolved_stats`](crate::theme_form::resolved_stats)
//!   (the C3 stat proof) — a text summary + an [`egui::ProgressBar`](bevy_egui::egui::ProgressBar) HP bar.
//! - RIGHT mode-form panel — the field stack: display name, the default-floor [`ComboBox`](bevy_egui::egui::ComboBox)
//!   (SLAB-ONLY via [`slab_floor_candidates`](crate::theme_form::slab_floor_candidates) — GTW-530
//!   C3), the read-only UUID, the New-theme button, and the debug-only Save button. The terrain
//!   library is NO LONGER here (it is the central primary region — GTW-530 C2).
//! - The `.terrain_theme.ron` preview is GONE (GTW-530 C1).
//!
//! ## Load-existing (C3.2)
//!
//! When the global theme `ComboBox` in the top bar selects a theme while in THEME mode, the shell
//! calls [`load_theme_into_form`] so the draft shows the selected theme's current definition. On
//! entering THEME mode with a theme already selected, the shell does the same check.

mod autoload;
mod fields;
mod library;
mod stats;

#[cfg(test)]
mod tests;

pub(crate) use autoload::{load_theme_into_form, resolve_autoload};
pub(crate) use fields::field_stack;
pub(crate) use library::terrain_library_panel;
pub(crate) use stats::stats_panel;

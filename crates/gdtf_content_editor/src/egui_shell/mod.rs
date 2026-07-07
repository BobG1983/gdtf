//! The editor's **egui Workbench shell** (GTW-512 C1) — the CLEAN SWAP off the hand-rolled
//! `bevy_ui` shell onto `bevy_egui` 0.41.
//!
//! Wiring-only module. The single egui UI system + its panel layout live in
//! [`shell`](self::shell); the global-theme `ComboBox` option builder lives in
//! [`theme_combo`](self::theme_combo); the real TERRAIN-mode form (GTW-513 C2) lives in
//! [`terrain_form_ui`](self::terrain_form_ui); the real THEME-mode form (GTW-514 C3) lives in
//! [`theme_form_ui`](self::theme_form_ui); the real PREFAB-mode form + the render-to-texture
//! viewport (GTW-515 C4) live in [`prefab`](self::prefab); the GANG-mode form (GTW-636) lives in
//! [`gang_form_ui`](self::gang_form_ui); the ARMOR-mode form (GTW-479) lives in
//! [`armor_form_ui`](self::armor_form_ui). The shell registers ONE UI system in the
//! [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) schedule (NOT `Update` —
//! bevy-traps: a `Update` system calling `ctx_mut()` fights the egui begin/end-pass plumbing),
//! gated `run_if(in_state(EditorState::Editing))`.
//!
//! The shell replicates the BEHAVIOR of the GTW-509 `bevy_ui` shell (the mode tabs + the global
//! theme dropdown + the status line + the four region panels), NOT its broken layout:
//!
//! - a top [`Panel`](bevy_egui::egui::Panel)`::top` — the `[TERRAIN | THEME | PREFAB]`
//!   mode tabs (driving the kept [`EditorMode`](crate::mode::EditorMode) resource) + the global
//!   theme [`ComboBox`](bevy_egui::egui::ComboBox) (folding a selection into the session exactly
//!   as the old `apply_theme_selection` did),
//! - a bottom [`Panel`](bevy_egui::egui::Panel)`::bottom` — the status line
//!   `"Mode: {LABEL}  |  Theme: {name}"`,
//! - a left [`Panel`](bevy_egui::egui::Panel)`::left` — the palette / stats placeholder,
//! - a right [`Panel`](bevy_egui::egui::Panel)`::right` — the ACTIVE mode's form (all three real:
//!   TERRAIN C2, THEME C3, PREFAB C4),
//! - a [`CentralPanel`](bevy_egui::egui::CentralPanel) LAST — the viewport (TERRAIN/THEME RON
//!   preview; PREFAB the render-to-texture tile viewport, C4).
//!
//! egui panels CANNOT overlap and the registration order is LOAD-BEARING (outermost-first, the
//! central panel last), so [`editor_egui_ui`](self::shell::editor_egui_ui) declares them in that
//! exact order.

// GTW-479: the ARMOR-mode form — the draw half over the `armor_form` model (the gang
// form split: model module + `*_form_ui` sibling).
mod armor_form_ui;
// The shell's mode-agnostic top/bottom-bar chrome (mode tabs + global theme combo +
// status line), split out of `shell.rs` (GTW-636 — module-layout bands).
mod chrome;
// GTW-636: the GANG-mode form — the draw half over the `gang_form` model (the terrain /
// theme form split: model module + `*_form_ui` sibling).
mod gang_form_ui;
// The shell system's per-mode model-borrow SystemParam bundles (PrefabParams /
// GangParams), split out of `shell.rs` (GTW-636 — module-layout bands).
mod params;
// `pub(crate)` (not private): lib.rs re-exports `prefab::size_fields::SizeFieldSpans` (GTW-464) so
// the headless integration test asserts the exact size-field view model the panel renders from.
pub(crate) mod prefab;
mod shell;
mod sprite_thumb;
mod terrain_form_ui;
mod theme_combo;
mod theme_form_ui;

pub(crate) use prefab::nav::{level_nav_hotkeys, view_mode_hotkey};
pub(crate) use shell::editor_egui_ui;

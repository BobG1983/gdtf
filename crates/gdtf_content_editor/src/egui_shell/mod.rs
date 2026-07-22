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
//! [`armor_form_ui`](self::armor_form_ui); the INJURY-mode forms (GTW-654 — the def editor +
//! the weighting section) live in [`injury_form_ui`](self::injury_form_ui); the SPRITE-mode
//! form (GTW-664 — source / visual anchor / facings / animation) lives in
//! [`sprite_form_ui`](self::sprite_form_ui); the ATTACHMENT-mode form (GTW-669 — display
//! name / slot / the closed 13-effect list) lives in
//! [`attachment_form_ui`](self::attachment_form_ui) over the SHARED
//! [`fire_mode_edit`](self::fire_mode_edit) row widget; the WEAPON-mode form (GTW-670 —
//! the full 18-field `WeaponSpec` in collapsible sections) lives in
//! [`weapon_form_ui`](self::weapon_form_ui) over the same shared widget; the
//! MELEE-mode form (GTW-671 — the full `MeleeWeaponSpec` in collapsible sections)
//! lives in [`melee_weapon_form_ui`](self::melee_weapon_form_ui) over the SHARED
//! [`damage_edit`](self::damage_edit) group + [`slots_edit`](self::slots_edit) lists
//! both weapon forms draw; the per-mode
//! RIGHT-form + CENTRAL-panel dispatches live in [`mode_panels`](self::mode_panels)
//! (the GTW-670 band boundary); the pre-panel
//! per-mode autoload/model-sync runners live in [`autoload`](self::autoload). The shell
//! registers ONE UI system in the
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
// GTW-669: the ATTACHMENT-mode form — the draw half over the `attachment_form` model
// (the gang / armor / sprite form split: model module + `*_form_ui` sibling).
mod attachment_form_ui;
// The shell's PRE-PANEL per-mode model-sync / autoload runners, split out of
// `shell.rs` at the GTW-479-flagged split point (GTW-654 — module-layout bands).
mod autoload;
// The shell's mode-agnostic top/bottom-bar chrome (mode tabs + global theme combo +
// status line), split out of `shell.rs` (GTW-636 — module-layout bands).
mod chrome;
// GTW-671: the SHARED damage-group editor — one authoring surface for the six fields
// the ranged and melee weapon specs share verbatim (damage / punch / shred /
// damage_type / fatal_bias / handedness), plus the generic scalar drags both forms'
// remaining fields reuse (the `fire_mode_edit` shared-leaf precedent).
mod damage_edit;
// GTW-669: the SHARED fire-mode row editor — one `FireModeSpec` authoring surface the
// ATTACHMENT mode's `GainFireMode` rows and the GTW-670 weapon forms both consume (the
// `theme_combo` / `sprite_thumb` shared-leaf precedent).
mod fire_mode_edit;
// GTW-636: the GANG-mode form — the draw half over the `gang_form` model (the terrain /
// theme form split: model module + `*_form_ui` sibling).
mod gang_form_ui;
// GTW-654: the INJURY-mode forms — the draw half over the `injury_form` models (the
// def editor + the C2 weighting section; the gang/armor form split).
mod injury_form_ui;
// GTW-671: the MELEE-mode form — the draw half over the `melee_weapon_form` model (the
// gang / armor / sprite / attachment / weapon form split: model module + `*_form_ui`
// sibling).
mod melee_weapon_form_ui;
// GTW-670: the per-mode RIGHT-form + CENTRAL-panel dispatches (one arm per Workbench
// mode), split out of `shell.rs` at the band boundary.
mod mode_panels;
// The shell system's per-mode model-borrow SystemParam bundles (PrefabParams /
// GangParams), split out of `shell.rs` (GTW-636 — module-layout bands).
mod params;
// `pub(crate)` (not private): lib.rs re-exports `prefab::size_fields::SizeFieldSpans` (GTW-464) so
// the headless integration test asserts the exact size-field view model the panel renders from.
pub(crate) mod prefab;
mod shell;
// GTW-671: the SHARED slot-declaration + attachment-key list editors — the GTW-670
// weapon-form widgets lifted to one authoring surface both weapon forms draw (the
// `fire_mode_edit` / `damage_edit` shared-leaf precedent).
mod slots_edit;
// GTW-664: the SPRITE-mode form — the draw half over the `sprite_form` model (the gang /
// armor form split: model module + `*_form_ui` sibling).
mod sprite_form_ui;
mod sprite_thumb;
mod terrain_form_ui;
// The shell's PRE-PANEL egui texture-id resolution, split out of `shell.rs` at the
// GTW-664 boundary (module-layout bands): it changes when a mode's TEXTURE surface does.
mod textures;
mod theme_combo;
mod theme_form_ui;
// GTW-670: the WEAPON-mode form — the draw half over the `weapon_form` model (the gang
// / armor / sprite / attachment form split: model module + `*_form_ui` sibling).
mod weapon_form_ui;

pub(crate) use prefab::nav::{level_nav_hotkeys, view_mode_hotkey};
pub(crate) use shell::editor_egui_ui;

//! The egui TERRAIN-mode authoring form (GTW-513 C2) — the real terrain form that replaces the
//! C1 stub in the right panel + left palette + central (RON-preview) regions of the egui shell.
//!
//! This module is the egui DRAW + the debug-only save press for the TERRAIN mode. It REUSES the
//! [`terrain_form`](crate::terrain_form) model + save VERBATIM (C2.2): every control reads / writes
//! the state-scoped [`TerrainDraft`](crate::terrain_form::TerrainDraft) resource through its
//! existing accessors / setters, and the save button calls the existing
//! [`write_terrain`](crate::terrain_form::write_terrain) after the idempotent
//! [`ensure_uuid`](crate::terrain_form::TerrainDraft::ensure_uuid) mint — round-tripping the
//! GTW-487 loader. The egui draw lives in
//! [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) (bevy-traps #8) via the
//! caller [`editor_egui_ui`](super::shell::editor_egui_ui); the controls mutate the draft in place
//! (no message round-trip — egui is immediate-mode), so the mutations are idempotent under the
//! multipass re-run (bevy-traps #8 fact (b)).
//!
//! ## Layout across the shell panels (GTW-534 C1/C2 — reworked emphasis)
//!
//! GTW-534 INVERTS the GTW-513 emphasis: authoring a terrain is about SETTING STATS and PICKING A
//! SPRITE, so those are now the CENTRAL / primary focus and the useful-but-secondary
//! `.terrain_def.ron` preview is demoted to a side strip (kept + live, not dominating):
//!
//! - CENTRAL primary panel — the [`primary_panel`](panel::primary_panel): the sprite-grid graphic
//!   picker (the GTW-516 [`graphic_picker`](panel::graphic_picker)) + the terrain
//!   [`field_stack`](fields::field_stack) side by side, the two authoring foci filling
//!   the largest / most-prominent region,
//! - LEFT secondary strip — the live monospace `.terrain_def.ron`
//!   [`ron_preview`](preview::ron_preview) (re-serialized from the draft each frame),
//! - RIGHT mode-form panel — idle in TERRAIN mode (the shell only draws it for THEME / PREFAB).
//!
//! ## GTW-574: the Emplacement authoring surface
//!
//! The kind segmented row now offers the FOURTH canonical kind, `Emplacement` (the pick list is
//! compiler-tied to `TerrainPieceKind` — GTW-574 C4), and the field stack gains the
//! Emplacement-ONLY mounted-weapon [`ComboBox`](bevy_egui::egui::ComboBox)
//! ([`fields::mounted_weapon_combo`]) backed by the LIVE
//! [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) (Q1 — a registry-backed dropdown,
//! never free-text), its options the registry keys SORTED for a stable order.
//!
//! Split by concern (GTW-574; the single `terrain_form_ui.rs` outgrew the file caps):
//! [`panel`] (central composition + the sprite picker), [`fields`] (the stat field stack + the
//! save press), [`preview`] (the demoted RON preview). Wiring-only here.

mod fields;
mod panel;
mod preview;

pub(crate) use panel::primary_panel;
pub(crate) use preview::ron_preview;

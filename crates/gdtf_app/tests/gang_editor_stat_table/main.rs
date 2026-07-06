//! GTW-428: headless behavioral tests for the in-app gang-editor's EXPANDED per-member stat table.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin` —
//! which registers `drive_accordions` — and the real `EditorScenePlugin` wired through
//! `ScenesPlugin`), seeded with a theme + a [`GangerStatTuning`] so the editor screen spawns with
//! the GTW-384 derivation available. They assert on the WORLD and the model resource (the
//! system-effects) — never on rendering or real device input.
//!
//! Editing is driven through the REAL input MESSAGES the widgets raise
//! ([`NumericFieldCommitted`]`<f32>`) or the real action system via a set [`Interaction::Pressed`]
//! plus an `update()` (the accepted headless idiom — `MinimalPlugins` has no `ui_focus_system` to
//! clobber it). Each assertion is pin-discriminating: it would FAIL if its clause were reverted.
//!
//! Coverage (the ticket's three headless tests):
//!
//! - [`pip_press_drives_the_accordion_target`] — toggling the pip flips its `PipExpanded` AND
//!   drives the matching stat panel's `AccordionAnim` toward `Expanding` (C1).
//! - [`expanded_panel_has_eight_clamped_attribute_fields`] — the panel holds the eight editable
//!   attribute fields, each carrying a `NumericRange<f32>` that clamps out-of-range input (C2).
//! - [`editing_attribute_recomputes_derived_to_pipeline_output`] — a real `NumericFieldCommitted`
//!   on an attribute updates the member attribute AND every displayed derived stat equals the
//!   GTW-384 `derive_stats` output for the new attributes (C3, pin-discriminating: a missing
//!   recompute or stale attributes would leave the OLD derived text and fail).

mod accordion;
mod attribute_fields;
mod harness;
mod panel_fit;

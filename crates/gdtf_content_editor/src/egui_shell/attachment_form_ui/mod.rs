//! The ATTACHMENT tab's egui form (GTW-669) — the DRAW half over the
//! [`attachment_form`](crate::attachment_form) model, following the gang / armor / sprite
//! form split (model module + `*_form_ui` sibling, GTW-636).
//!
//! Wiring-only module. The one-shot open-with-an-item seed lives in [`autoload`]; the
//! RIGHT-panel field stack (load `ComboBox`, name field, New attachment / debug-only
//! Save) lives in [`fields`]; the CENTRAL primary panel (display name, the closed 6-slot
//! combo, and the EFFECTS LIST over the closed 13-effect palette — GTW-669 C2) lives in
//! [`def_panel`], whose `GainFireMode` rows draw the SHARED
//! [`fire_mode_edit`](super::fire_mode_edit) widget.

mod autoload;
mod def_panel;
mod fields;

pub(crate) use autoload::autoload_first_attachment;
pub(crate) use def_panel::def_panel;
pub(crate) use fields::field_stack;

//! The ARMOR tab's egui form (GTW-479) — the DRAW half over the
//! [`armor_form`](crate::armor_form) model, following the gang form split
//! (model module + `*_form_ui` sibling, GTW-636).
//!
//! Wiring-only module. The one-shot open-with-an-armor seed lives in [`autoload`]; the
//! RIGHT-panel field stack (load `ComboBox`, name field, New armor / debug-only Save)
//! lives in [`fields`]; the CENTRAL per-body-part piece editor (one row per
//! [`BodyPart`](gdtf_battle_sim::armor::BodyPart): the four stat drags + the
//! `armor_type` combo) lives in [`pieces`].

mod autoload;
mod fields;
mod pieces;

pub(crate) use autoload::autoload_first_armor;
pub(crate) use fields::field_stack;
pub(crate) use pieces::pieces_panel;

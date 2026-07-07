//! The GANG tab's egui form (GTW-636) — the DRAW half over the
//! [`gang_form`](crate::gang_form) model, following the terrain / theme form split
//! (model module + `*_form_ui` sibling).
//!
//! Wiring-only module. The one-shot open-with-a-gang seed lives in [`autoload`]; the
//! RIGHT-panel field stack (load `ComboBox`, name field, New gang / Add member /
//! debug-only Save) lives in [`fields`]; the CENTRAL member-list editor (per-member
//! name / loadout / attributes / derived stats / remove) lives in [`members`].

mod autoload;
mod fields;
mod members;

pub(crate) use autoload::autoload_first_gang;
pub(crate) use fields::field_stack;
pub(crate) use members::members_panel;

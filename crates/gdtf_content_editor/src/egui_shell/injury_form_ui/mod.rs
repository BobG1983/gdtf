//! The INJURY tab's egui form (GTW-654) — the DRAW half over the
//! [`injury_form`](crate::injury_form) models, following the gang/armor form split
//! (model module + `*_form_ui` sibling, GTW-636/GTW-479).
//!
//! Wiring-only module. The one-shot open-with-content seeds live in [`autoload`];
//! the RIGHT-panel field stack (load `ComboBox`, key field, New injury /
//! debug-only Save) lives in [`fields`]; the CENTRAL def editor (display name /
//! category / severity / texts + the closed-palette EFFECTS list with add/remove
//! rows) lives in [`def_panel`]; the CENTRAL weighting section (context-table
//! combo + per-severity rows whose injury names come from the registry, GTW-654
//! C2) lives in [`weighting_panel`].

mod autoload;
mod def_panel;
mod fields;
mod weighting_panel;

pub(crate) use autoload::{autoload_first_injury, autoload_weighting_table};
pub(crate) use def_panel::def_panel;
pub(crate) use fields::field_stack;
pub(crate) use weighting_panel::weighting_panel;

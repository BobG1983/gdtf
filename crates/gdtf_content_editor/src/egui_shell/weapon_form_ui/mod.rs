//! The WEAPON tab's egui form (GTW-670) — the DRAW half over the
//! [`weapon_form`](crate::weapon_form) model, following the gang / armor / sprite /
//! attachment form split (model module + `*_form_ui` sibling, GTW-636).
//!
//! Wiring-only module. The one-shot open-with-a-weapon seed lives in [`autoload`]; the
//! RIGHT-panel field stack (load `ComboBox`, name field, New weapon / debug-only Save)
//! lives in [`fields`]; the CENTRAL primary panel — the full 18-field
//! [`WeaponSpec`](gdtf_battle_sim::weapon::WeaponSpec) editor in collapsible sections
//! (GTW-670 C2) — is the [`def_panel`] skeleton over the per-concern section bodies:
//! [`stats`] (the seven scalars + the handling combos/tags + the magazine pair),
//! [`lists`] (fire modes via the SHARED
//! [`fire_mode_edit`](super::fire_mode_edit) widget, slots, and registry-sourced
//! attachment keys), and [`optionals`] (the enable-gated dot / on-death sub-forms).

mod autoload;
mod def_panel;
mod fields;
mod lists;
mod optionals;
mod stats;

pub(crate) use autoload::autoload_first_weapon;
pub(crate) use def_panel::def_panel;
pub(crate) use fields::field_stack;

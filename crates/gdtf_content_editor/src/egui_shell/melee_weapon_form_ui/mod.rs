//! The MELEE tab's egui form (GTW-671) — the DRAW half over the
//! [`melee_weapon_form`](crate::melee_weapon_form) model, following the gang / armor /
//! sprite / attachment / weapon form split (model module + `*_form_ui` sibling,
//! GTW-636).
//!
//! Wiring-only module. The one-shot open-with-a-weapon seed lives in [`autoload`]; the
//! RIGHT-panel field stack (load `ComboBox`, name field, New melee weapon / debug-only
//! Save) lives in [`fields`]; the CENTRAL primary panel — the full
//! [`MeleeWeaponSpec`](gdtf_battle_sim::weapon::MeleeWeaponSpec) editor in collapsible
//! sections (GTW-671 C2) — is the [`def_panel`] skeleton over the SHARED
//! [`damage_edit`](super::damage_edit) group (one authoring surface for the six fields
//! the ranged spec shares verbatim), the melee-only [`fight_modes`] rows (the melee
//! sibling of the shared fire-mode row — a distinct spec type, the same look), and the
//! SHARED [`slots_edit`](super::slots_edit) slot / attachment lists.

mod autoload;
mod def_panel;
mod fields;
mod fight_modes;

pub(crate) use autoload::autoload_first_melee_weapon;
pub(crate) use def_panel::def_panel;
pub(crate) use fields::field_stack;

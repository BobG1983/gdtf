//! `editor.set_field` — write one single-value field of the active mode's draft.

mod armor;
mod attachment;
mod command;
mod field;
mod gang;
mod injury;
mod melee_weapon;
mod route;
mod sprite;
mod terrain;
mod terrain_on_death;
mod weapon;
mod weighting;

pub(in crate::net_qa) use command::EditorSetField;

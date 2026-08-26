//! `editor.set_field` — write one single-value field of the active mode's draft.

mod armor;
mod attachment;
mod command;
mod injury;
mod melee_weapon;
mod sprite;
mod terrain;

pub(in crate::net_qa) use command::EditorSetField;

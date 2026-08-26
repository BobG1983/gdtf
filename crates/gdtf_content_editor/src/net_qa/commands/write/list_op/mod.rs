//! `editor.list_op` — edit one list-valued field of the active mode's draft.

mod attachment;
mod command;
mod field;
mod gang;
mod injury;
mod melee_weapon;
mod shared;
mod sprite;
mod terrain;
mod weapon;

pub(in crate::net_qa) use command::EditorListOp;

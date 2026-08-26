//! `editor.list_op` — edit one list-valued field of the active mode's draft.

mod attachment;
mod command;
mod sprite;
mod terrain;

pub(in crate::net_qa) use command::EditorListOp;

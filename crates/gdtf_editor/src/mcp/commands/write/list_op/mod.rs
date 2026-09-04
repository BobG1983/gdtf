//! `editor.list_op` — edit one list-valued field of the active mode's draft.

mod attachment;
mod command;
mod field;
mod gang;
mod injury;
mod melee_weapon;
mod route;
pub(in crate::mcp::commands::write) mod shared;
mod sprite;
mod terrain;
mod weapon;
mod weighting;

pub(in crate::mcp) use command::EditorListOp;

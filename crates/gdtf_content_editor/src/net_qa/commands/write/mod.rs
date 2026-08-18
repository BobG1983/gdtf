//! Editor writes that change a draft, a mode tab, or a file on disk.

mod availability;
mod blank;
mod list_op;
mod load;
mod save;
mod set_field;
mod set_mode;

pub(in crate::net_qa) use blank::EditorNew;
pub(in crate::net_qa) use list_op::EditorListOp;
pub(in crate::net_qa) use load::EditorLoad;
pub(in crate::net_qa) use save::EditorSave;
pub(in crate::net_qa) use set_field::EditorSetField;
pub(in crate::net_qa) use set_mode::EditorSetMode;

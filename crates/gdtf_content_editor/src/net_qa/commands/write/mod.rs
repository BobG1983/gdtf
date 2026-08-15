//! Editor writes that change a draft, a mode tab, or a file on disk.

mod availability;
mod blank;
mod load;
mod save;
mod set_mode;

pub(in crate::net_qa) use blank::EditorNew;
pub(in crate::net_qa) use load::EditorLoad;
pub(in crate::net_qa) use save::EditorSave;
pub(in crate::net_qa) use set_mode::EditorSetMode;

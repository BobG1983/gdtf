//! `editor.save` and the root-taking writers it calls, one per family.

mod command;
mod dispatch;
mod families;
mod map_forms;

pub(in crate::mcp) use command::EditorSave;

//! The editor's typed command layer: the set, its registration, its conformance checks.

mod conformance;
mod read;
mod register;
mod set;

pub use conformance::{assert_editor_command_set_is_conformant, editor_command_names};
pub(in crate::net_qa) use register::register_editor_commands;
pub(in crate::net_qa) use set::EDITOR_COMMANDS;

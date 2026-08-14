//! The one list of commands the editor host publishes.

use gdtf_qa_command::command::ErasedCommand;

use super::read::EditorPhase;
use crate::net_qa::facts::EditorFacts;

pub(in crate::net_qa) const EDITOR_COMMANDS: &[&dyn ErasedCommand<EditorFacts>] = &[&EditorPhase];

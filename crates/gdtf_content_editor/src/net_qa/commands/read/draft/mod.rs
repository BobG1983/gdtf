//! `editor.draft`: the active mode's draft, as the RON text a save would write.

mod command;
mod drafts;
mod project;

pub(in crate::net_qa) use command::EditorDraft;

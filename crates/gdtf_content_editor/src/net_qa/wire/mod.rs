//! Wire mirrors of the editor's own lifecycle state, mode tabs, and write outcomes.

mod cell;
mod key;
mod last_save;
mod mode;
mod outcome;
mod phase;
mod refusal;
mod save_fault;
#[cfg(test)]
mod test;

pub(in crate::net_qa) use key::{EditorContentNameNet, EditorKeyNet, SavedPathNet};
pub(in crate::net_qa) use last_save::{EditorLastSaveRowNet, LastSaveOutcomeNet};
pub(in crate::net_qa) use mode::EditorModeNet;
pub(in crate::net_qa) use outcome::{
    EditorLoadOutcomeNet, EditorNewOutcomeNet, EditorSaveOutcomeNet,
};
pub(in crate::net_qa) use phase::EditorPhaseNet;
pub(in crate::net_qa) use refusal::EditorRefusalNet;
pub(in crate::net_qa) use save_fault::EditorSaveFaultNet;

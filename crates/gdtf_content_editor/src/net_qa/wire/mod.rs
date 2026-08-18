//! Wire mirrors of the editor's own lifecycle state, mode tabs, and write outcomes.

mod cell;
mod facing;
mod field;
mod key;
mod last_save;
mod list;
mod mode;
mod outcome;
mod phase;
mod refusal;
mod save_fault;
mod terrain_kind;
#[cfg(test)]
mod test;

pub(in crate::net_qa) use facing::TerrainFacingNet;
pub(in crate::net_qa) use field::EditorFieldNet;
pub(in crate::net_qa) use key::{EditorContentNameNet, EditorKeyNet, SavedPathNet};
pub(in crate::net_qa) use last_save::{EditorLastSaveRowNet, LastSaveOutcomeNet};
pub(in crate::net_qa) use list::{EditorListMemberNet, EditorListNet, EditorListOpNet};
pub(in crate::net_qa) use mode::EditorModeNet;
pub(in crate::net_qa) use outcome::{
    EditorLoadOutcomeNet, EditorNewOutcomeNet, EditorSaveOutcomeNet,
};
pub(in crate::net_qa) use phase::EditorPhaseNet;
pub(in crate::net_qa) use refusal::EditorRefusalNet;
pub(in crate::net_qa) use save_fault::EditorSaveFaultNet;
pub(in crate::net_qa) use terrain_kind::TerrainKindNet;

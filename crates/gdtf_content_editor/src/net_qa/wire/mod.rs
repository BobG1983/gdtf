//! Wire mirrors of the editor's own lifecycle state, mode tabs, and write outcomes.

mod camera;
mod cell;
mod draft;
mod facing;
mod family;
mod field;
mod grid;
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
mod toggle;
mod validation;
mod view;

pub(in crate::net_qa) use camera::{EditorPanNet, EditorZoomNet};
pub(in crate::net_qa) use cell::EditorLevelNet;
pub(in crate::net_qa) use draft::{EditorDraftOutcomeNet, EditorDraftRonNet};
pub(in crate::net_qa) use facing::TerrainFacingNet;
pub(in crate::net_qa) use family::{
    EditorFamilyEntryNet, EditorFamilyLabelNet, EditorFamilyNet, EditorFamilyRowNet,
};
pub(in crate::net_qa) use field::EditorFieldNet;
pub(in crate::net_qa) use grid::EditorGridSizeNet;
pub(in crate::net_qa) use key::{
    EditorContentNameNet, EditorKeyNet, SavedPathNet, TerrainKeyNet, ThemeKeyNet,
};
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
pub(in crate::net_qa) use toggle::TerrainToggleNet;
pub(in crate::net_qa) use validation::{
    ChecksCompleteNet, ValidationFindingNet, ValidationPublishedNet,
};
pub(in crate::net_qa) use view::{EditorIsolateViewNet, EditorViewModeNet, EditorViewNet};

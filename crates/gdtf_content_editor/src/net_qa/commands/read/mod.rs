//! Editor reads that answer from the world without changing it.

mod editor_phase;
mod last_save;

pub(in crate::net_qa) use editor_phase::EditorPhase;
pub(in crate::net_qa) use last_save::EditorLastSave;

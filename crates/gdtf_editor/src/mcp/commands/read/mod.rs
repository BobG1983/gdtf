//! Editor reads that answer from the world without changing it.

mod draft;
mod editor_phase;
mod families;
mod last_save;
mod painted_map;
mod session;
mod validation;
mod weighting;

pub(in crate::mcp) use draft::EditorDraft;
pub(in crate::mcp) use editor_phase::EditorPhase;
pub(in crate::mcp) use families::EditorFamilies;
pub(in crate::mcp) use last_save::EditorLastSave;
pub(in crate::mcp) use painted_map::EditorPaintedMap;
pub(in crate::mcp) use session::EditorSession;
pub(in crate::mcp) use validation::EditorValidation;
pub(in crate::mcp) use weighting::EditorWeighting;

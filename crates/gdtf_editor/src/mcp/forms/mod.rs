//! The drafts and registries an editor write command reads and writes.

mod drafts;
mod registries;

pub(in crate::mcp) use drafts::EditorForms;
pub(in crate::mcp) use registries::EditorRegistries;

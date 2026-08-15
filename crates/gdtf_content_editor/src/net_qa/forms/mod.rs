//! The drafts and registries an editor write command reads and writes.

mod drafts;
mod registries;

pub(in crate::net_qa) use drafts::EditorForms;
pub(in crate::net_qa) use registries::EditorRegistries;

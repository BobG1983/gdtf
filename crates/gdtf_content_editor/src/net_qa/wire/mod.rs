//! Wire mirrors of the editor's own lifecycle state and mode tabs.

mod mode;
mod phase;
#[cfg(test)]
mod test;

pub(in crate::net_qa) use mode::EditorModeNet;
pub(in crate::net_qa) use phase::EditorPhaseNet;

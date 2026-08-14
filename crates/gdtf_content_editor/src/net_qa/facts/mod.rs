//! Host facts an editor command's availability check reads.

mod editor_facts;
mod read;

pub(in crate::net_qa) use editor_facts::EditorFacts;
pub(in crate::net_qa) use read::EditorFactsParam;

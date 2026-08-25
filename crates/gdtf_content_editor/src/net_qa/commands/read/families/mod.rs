//! `editor.families`: the registry keys an author can pick, and the label beside each.

mod collect;
mod command;
mod sort;
#[cfg(test)]
mod test;

pub(in crate::net_qa) use command::EditorFamilies;

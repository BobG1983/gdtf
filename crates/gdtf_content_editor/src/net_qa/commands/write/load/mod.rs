//! `editor.load` and the per-family registry lookups it calls.

mod command;
mod dispatch;
mod families;
#[cfg(test)]
mod test;
mod theme;

pub(in crate::net_qa) use command::EditorLoad;

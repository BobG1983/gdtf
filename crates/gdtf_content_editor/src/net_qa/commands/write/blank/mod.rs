//! `editor.new` and the per-form blank-draft constructors it calls.

mod command;
mod families;
mod refusal;
#[cfg(test)]
mod test;

pub(in crate::net_qa) use command::EditorNew;

//! The editor host's command that holds its reply until a named condition comes true.
mod command;
mod probe;

#[cfg(test)]
mod test;

pub(in crate::net_qa) use command::EditorWait;
pub use command::shorten_editor_wait_budget;

//! The Gang form's own field arms, written through the draft its widgets write.

mod registry;
#[cfg(test)]
mod test;
mod write;

pub(super) use write::write;

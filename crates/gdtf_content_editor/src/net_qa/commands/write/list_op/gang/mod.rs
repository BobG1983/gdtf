//! The Gang form's own member list, edited the way its Add and Remove buttons edit it.

mod roster;
#[cfg(test)]
mod test;

pub(super) use roster::{apply, members};

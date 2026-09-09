//! When a wait runs out. A wait that never expires answers no deadline instead of overflowing.

mod at;
mod grace;
#[cfg(test)]
mod test;

pub(super) use at::DeadlineAt;
pub(super) use grace::grace_deadline;

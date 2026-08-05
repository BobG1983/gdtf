//! Command that holds its reply until a named condition comes true.
pub(crate) mod command;
pub(crate) mod probe;

pub(in crate::dev::net_qa) use command::Wait;
#[cfg(feature = "headless_test")]
pub use command::shorten_wait_budget;

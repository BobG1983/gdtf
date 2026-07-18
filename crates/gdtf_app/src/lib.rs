//! Bevy app for GDTF.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
pub use app::GdtfApp;

// The DEV-ONLY QA affordance stack (GTW-632, extended GTW-749): the `net_qa` network
// control channel is the ONE capture / drive path — owned here, wired into `GdtfApp`
// through the one `DevAffordancesPlugin` aggregate.
mod dev;

mod states;

#[cfg(feature = "test-support")]
pub mod test_support;

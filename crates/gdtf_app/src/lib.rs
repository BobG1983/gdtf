//! Bevy app for GDTF.

mod support;
pub(crate) use support::{support_item, support_use};

mod app;
pub use app::GdtfApp;

// The DEV-ONLY QA affordance stack (GTW-632, extended GTW-749): the `net_qa` network
// control channel is the ONE capture / drive path — owned here, wired into `GdtfApp`
// through the one `DevAffordancesPlugin` aggregate.
mod dev;

// GTW-944: the QA host's wire vocabulary, re-exported so it has a public path.
//
// It is the type floor the twelve game-command tickets are built on, so it lands complete
// and most of it has no consumer inside this crate yet. `dead_code` fires on an unconsumed
// `pub(crate)` type in a private module, and GTW-944 forbids silencing that lint, so the
// vocabulary is expressed by its VISIBILITY instead: `pub` types reachable from the crate
// root, exercised by the round-trip and schema suites in `dev::net_qa::wire::test`. The
// `phase` mirrors are the exception and stay crate-scoped — the `app.phase` command already
// consumes them, so they never read as dead.
//
// It is public only where the QA listener itself is compiled: `net_qa` is opt-in, never in
// the default feature set, never named for a release build, and the whole module is
// additionally `debug_assertions`-gated. A shipped `gdtf_app` still has exactly one public
// item, `GdtfApp`.
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub use dev::net_qa::wire as qa_wire;

mod states;

#[cfg(feature = "test-support")]
pub mod test_support;

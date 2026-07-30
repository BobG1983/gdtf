//! That the PRODUCTION constructors carry the real orphan watch (GTW-926).
//!
//! [`orphan`](super::orphan) drives the manager's orphan decisions through
//! [`HostManager::with_orphan_watch`], which every test there hands a watch of its own so
//! nothing signals the test runner. That leaves one line untested: the watch
//! [`HostManager::with_config`] supplies — and it is the only line connecting all of those
//! decisions to the shipped MCP host (`serve.rs` builds both managers through it). A watch
//! swapped out there for one that answers "free" would leave every other test green and put
//! the shipped host back to answering `not_running` over a live orphan.
//!
//! Only the LAUNCH path can pin it: the launch reports an orphan and signals nothing
//! ([`manager`](gdtf_qa_mcp::HostManager) never stops a child it did not spawn on that
//! path), while a stop through the production watch would send a real `kill` to the process
//! holding the port — which, in a test, is the test runner itself.

use gdtf_qa_mcp::{
    HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, QaHost, QaPort, StopOutcome,
};

use super::support::{StubSpawner, fast_config, free_port, spawn_fake_game};

/// A manager built the way the shipped host builds one — [`HostManager::with_config`], with
/// no watch supplied — sees a held port and fails the launch on it.
///
/// Nothing here injects an [`OrphanWatch`](gdtf_qa_mcp::OrphanWatch): the probe and the
/// process lookup are the production [`SystemOrphanWatch`](gdtf_qa_mcp::SystemOrphanWatch),
/// against a real listener answering the QA handshake on a real port.
#[test]
fn the_production_constructor_sees_a_held_port() {
    let port = QaPort::new(spawn_fake_game());
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(500));

    let outcome = manager.launch(port, &QaHost::Game.default_spec());

    let LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { port: reported, .. }) = outcome
    else {
        unreachable!(
            "the manager the shipped host builds reports the orphan holding the port, got: \
             {outcome:?}"
        );
    };
    assert_eq!(reported, port);
}

/// The same production-built manager over a FREE port launches normally — so the test above
/// pins the watch answering about a real hold, not a constructor that fails every launch.
#[test]
fn the_production_constructor_still_launches_on_a_free_port() {
    let port = QaPort::new(free_port());
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(200));

    let outcome = manager.launch(port, &QaHost::Game.default_spec());

    // Nothing answers on that port, so the launch runs to its boot timeout — the point is
    // that it got PAST the orphan check and spawned, which `PortHeldByOrphan` never does.
    assert!(
        matches!(
            outcome,
            LaunchOutcome::Failed(LaunchFailure::Timeout { .. })
        ),
        "a free port lets the launch proceed to the readiness wait, got: {outcome:?}"
    );
    assert_eq!(manager.stop_owned(), StopOutcome::NotRunning);
}

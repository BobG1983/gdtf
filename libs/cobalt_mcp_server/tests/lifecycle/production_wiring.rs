use cobalt_mcp_server::{
    HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, QaPort, StopOutcome,
};

use super::support::{StubSpawner, fast_config, free_port, sample_spec, spawn_fake_game};

#[test]
fn the_production_constructor_sees_a_held_port() {
    let port = QaPort::new(spawn_fake_game());
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(500));

    let outcome = manager.launch(port, &sample_spec());

    let LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { port: reported, .. }) = outcome
    else {
        unreachable!(
            "the manager the shipped host builds reports the orphan holding the port, got: \
             {outcome:?}"
        );
    };
    assert_eq!(reported, port);
}

#[test]
fn the_production_constructor_still_launches_on_a_free_port() {
    let port = QaPort::new(free_port());
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(200));

    let outcome = manager.launch(port, &sample_spec());

    assert!(
        matches!(
            outcome,
            LaunchOutcome::Failed(LaunchFailure::Timeout { .. })
        ),
        "a free port lets the launch proceed to the readiness wait, got: {outcome:?}"
    );
    assert_eq!(manager.stop_owned(), StopOutcome::NotRunning);
}

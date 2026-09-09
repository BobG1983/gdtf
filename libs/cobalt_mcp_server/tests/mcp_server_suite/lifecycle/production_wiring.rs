use std::{
    io,
    sync::{Arc, Mutex},
};

use cobalt_mcp_server::{
    ChildSpawner, HostLifecycle, HostManager, LaunchFailure, LaunchOutcome, LaunchSpec,
    ManagedChild, McpPort, StopOutcome,
};

use super::support::{StubSpawner, fast_config, free_port, sample_spec, spawn_fake_game};

type SpawnLog = Arc<Mutex<Vec<McpPort>>>;

struct RecordingSpawner {
    spawned: SpawnLog,
}

impl ChildSpawner for RecordingSpawner {
    fn spawn(&self, port: McpPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        if let Ok(mut log) = self.spawned.lock() {
            log.push(port);
        }
        StubSpawner.spawn(port, spec)
    }
}

fn recorded(log: &SpawnLog) -> Vec<McpPort> {
    log.lock().map(|ports| ports.clone()).unwrap_or_default()
}

#[test]
fn the_production_constructor_sees_a_held_port() {
    let port = McpPort::new(spawn_fake_game());
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
    let port = McpPort::new(free_port());
    let spawned: SpawnLog = Arc::new(Mutex::new(Vec::new()));
    // A zero boot wait expires on the first pass whatever the machine is doing.
    let mut manager = HostManager::with_config(
        Box::new(RecordingSpawner {
            spawned: Arc::clone(&spawned),
        }),
        fast_config(0),
    );

    let outcome = manager.launch(port, &sample_spec());

    assert_eq!(
        recorded(&spawned),
        vec![port],
        "a free port let the launch reach the spawner"
    );
    assert!(
        matches!(
            outcome,
            LaunchOutcome::Failed(LaunchFailure::Timeout { .. })
        ),
        "a free port lets the launch proceed to the readiness wait, got: {outcome:?}"
    );
    assert_eq!(manager.stop_owned(), StopOutcome::NotRunning);
}

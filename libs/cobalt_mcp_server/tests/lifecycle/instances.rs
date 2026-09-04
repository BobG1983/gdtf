use std::sync::{Arc, Mutex};

use cobalt_mcp_server::{
    ChildPid, HostLifecycle, HostManager, InstanceId, LaunchOutcome, LaunchPolicy, LaunchSpec,
    LifecycleConfig, McpPort, RecordedInstance, StopOutcome,
};

use crate::{
    fake_child::{CallLog, ChildCall, PortGatedSpawner, counted, recorded},
    support::{
        WatchFreePort, always_spawning_config, fast_config_with_policy, gated_listeners,
        recipe_with_features, sample_spec,
    },
};

const BOOT_MS: u64 = 2000;

struct Fixture {
    manager: HostManager,
    calls:   CallLog,
    ports:   Vec<McpPort>,
}

fn manager_over_gated_listeners(count: usize) -> Fixture {
    manager_configured_over_gated_listeners(count, always_spawning_config(BOOT_MS))
}

fn manager_configured_over_gated_listeners(count: usize, config: LifecycleConfig) -> Fixture {
    let gates = gated_listeners(count);
    let ports: Vec<McpPort> = gates.iter().map(|(port, _)| *port).collect();
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let manager = HostManager::with_orphan_watch(
        Box::new(PortGatedSpawner::new(Arc::clone(&calls), gates)),
        config,
        Box::new(WatchFreePort),
    );
    Fixture {
        manager,
        calls,
        ports,
    }
}

fn port_at(fixture: &Fixture, at: usize) -> McpPort {
    let Some(port) = fixture.ports.get(at) else {
        unreachable!(
            "the fixture opened a listener at {at}, ports: {:?}",
            fixture.ports
        );
    };
    *port
}

fn launch_at(fixture: &mut Fixture, at: usize, spec: &LaunchSpec) -> LaunchOutcome {
    let port = port_at(fixture, at);
    fixture.manager.launch(port, spec)
}

fn launched(outcome: LaunchOutcome, which: &str) -> (InstanceId, McpPort) {
    let LaunchOutcome::Launched { instance, port, .. } = outcome else {
        unreachable!("the {which} launch becomes ready, got: {outcome:?}");
    };
    (instance, port)
}

fn launched_child(outcome: LaunchOutcome, which: &str) -> (InstanceId, ChildPid) {
    let LaunchOutcome::Launched { instance, pid, .. } = outcome else {
        unreachable!(
            "the {which} launch starts a child rather than answering from the record, got: \
             {outcome:?}"
        );
    };
    (instance, pid)
}

fn holding(recorded: &[RecordedInstance], id: &InstanceId) -> Option<RecordedInstance> {
    recorded
        .iter()
        .find(|instance| instance.id() == id)
        .cloned()
}

#[test]
fn two_launches_record_two_instances_on_two_ports() {
    let mut fixture = manager_over_gated_listeners(2);

    let first = launch_at(&mut fixture, 0, &sample_spec());
    let second = launch_at(&mut fixture, 1, &sample_spec());

    let LaunchOutcome::Launched {
        instance: first_id,
        pid: earlier_child,
        ..
    } = first
    else {
        unreachable!("the first launch becomes ready, got: {first:?}");
    };
    let LaunchOutcome::Launched {
        instance: second_id,
        pid: later_child,
        ..
    } = second
    else {
        unreachable!("the second launch spawns rather than reusing the record, got: {second:?}");
    };
    assert_ne!(first_id, second_id, "each launch mints its own instance id");
    assert_ne!(
        earlier_child, later_child,
        "each launch records its own child"
    );

    let held = fixture.manager.instances();
    assert_eq!(held.len(), 2, "both launches are recorded, got: {held:?}");
    let (Some(first_record), Some(second_record)) =
        (holding(&held, &first_id), holding(&held, &second_id))
    else {
        unreachable!("both ids are recorded, got: {held:?}");
    };
    assert_ne!(
        first_record.port(),
        second_record.port(),
        "the two instances listen on two ports, got: {held:?}"
    );
    assert_eq!(
        first_record.port(),
        port_at(&fixture, 0),
        "each record holds the port its own launch was handed, got: {held:?}"
    );
    assert_eq!(second_record.port(), port_at(&fixture, 1), "got: {held:?}");
}

#[test]
fn an_always_spawning_host_starts_a_second_child() {
    let mut fixture = manager_configured_over_gated_listeners(
        2,
        fast_config_with_policy(
            BOOT_MS,
            LifecycleConfig::defaults_with_policy(LaunchPolicy::AlwaysSpawn).launch_policy(),
        ),
    );

    let (first_instance, first_child) =
        launched_child(launch_at(&mut fixture, 0, &sample_spec()), "first child");
    let (second_instance, second_child) =
        launched_child(launch_at(&mut fixture, 1, &sample_spec()), "second child");

    assert_ne!(
        first_instance, second_instance,
        "an always-spawning host mints an id per launch"
    );
    assert_ne!(
        first_child, second_child,
        "and starts a child per launch rather than reporting the recorded one"
    );
    let held = fixture.manager.instances();
    assert_eq!(held.len(), 2, "both launches are recorded, got: {held:?}");
}

#[test]
fn the_always_spawn_policy_starts_a_second_recipe_instead_of_refusing_it() {
    let mut fixture = manager_over_gated_listeners(2);

    let first = launch_at(
        &mut fixture,
        0,
        &recipe_with_features(&["dynamic_linking", "dev_tools"]),
    );
    let second = launch_at(
        &mut fixture,
        1,
        &recipe_with_features(&["dynamic_linking", "file_watcher"]),
    );

    assert!(
        matches!(first, LaunchOutcome::Launched { .. }),
        "the first launch becomes ready, got: {first:?}"
    );
    assert!(
        matches!(second, LaunchOutcome::Launched { .. }),
        "under the always-spawn policy a second recipe starts rather than being compared to \
         the first, got: {second:?}"
    );
}

#[test]
fn stopping_one_instance_leaves_the_other_recorded() {
    let mut fixture = manager_over_gated_listeners(2);
    let (first_id, _) = launched(launch_at(&mut fixture, 0, &sample_spec()), "first");
    let (second_id, _) = launched(launch_at(&mut fixture, 1, &sample_spec()), "second");
    let held = fixture.manager.instances();
    let Some(second_before) = holding(&held, &second_id) else {
        unreachable!("the second launch is recorded, got: {held:?}");
    };

    let stopped = fixture.manager.stop_instance(&first_id);

    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "stopping a recorded instance reports the child it stopped, got: {stopped:?}"
    );
    let left = fixture.manager.instances();
    assert!(
        holding(&left, &first_id).is_none(),
        "the stopped instance is no longer recorded, still recorded: {left:?}"
    );
    assert_eq!(
        holding(&left, &second_id).map(|instance| instance.pid()),
        Some(second_before.pid()),
        "the other instance is still recorded, with its own pid, recorded: {left:?}"
    );
    assert_eq!(
        left.len(),
        1,
        "one stop removes one record, recorded: {left:?}"
    );
}

#[test]
fn host_shutdown_stops_every_recorded_instance() {
    let mut fixture = manager_over_gated_listeners(2);
    let first = launch_at(&mut fixture, 0, &sample_spec());
    let second = launch_at(&mut fixture, 1, &sample_spec());
    assert!(
        matches!(first, LaunchOutcome::Launched { .. }),
        "the first launch becomes ready, got: {first:?}"
    );
    assert!(
        matches!(second, LaunchOutcome::Launched { .. }),
        "the second launch becomes ready, got: {second:?}"
    );
    assert_eq!(
        fixture.manager.instances().len(),
        2,
        "both launches are recorded before the shutdown"
    );

    let stopped = fixture.manager.stop_owned();

    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "shutdown reports what it stopped, got: {stopped:?}"
    );
    assert_eq!(
        counted(&fixture.calls, ChildCall::Terminate),
        2,
        "both children were signalled, calls: {:?}",
        recorded(&fixture.calls)
    );
    assert_eq!(
        counted(&fixture.calls, ChildCall::Reap),
        2,
        "both children were reaped, calls: {:?}",
        recorded(&fixture.calls)
    );
    assert!(
        fixture.manager.instances().is_empty(),
        "shutdown leaves no record behind, recorded: {:?}",
        fixture.manager.instances()
    );
}

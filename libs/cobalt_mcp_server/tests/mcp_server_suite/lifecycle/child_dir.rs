use std::sync::{Arc, Mutex};

use cobalt_mcp_server::{
    CargoPackage, FeatureList, HostLifecycle, HostManager, LaunchOutcome, LaunchSpec, McpPort,
    StopOutcome, WorkingDir,
};

use crate::lifecycle::{
    fake_child::{CallLog, PortGatedSpawner},
    support::{
        GatedStubSpawner, SAMPLE_PACKAGE, WatchFreePort, always_spawning_no_boot_deadline_config,
        gated_listeners, no_boot_deadline_config, sample_spec, spawn_gated_fake_game,
    },
};

fn recipe_in(dir: &WorkingDir) -> LaunchSpec {
    LaunchSpec::new(
        CargoPackage::new(SAMPLE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(dir.clone()),
    )
}

#[test]
fn a_running_child_reports_the_directory_its_recipe_named() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager = HostManager::with_orphan_watch(
        Box::new(GatedStubSpawner::new(gate)),
        no_boot_deadline_config(),
        Box::new(WatchFreePort),
    );
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    let elsewhere = WorkingDir::new(std::env::temp_dir());
    assert_ne!(
        *elsewhere, here,
        "the recipe's directory differs from the host's, or this proves nothing",
    );

    assert_eq!(
        manager.child_working_dir(),
        None,
        "with no child running there is no child directory to report",
    );

    let outcome = manager.launch(McpPort::new(port), &recipe_in(&elsewhere));
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );
    assert_eq!(
        manager.child_working_dir(),
        Some(elsewhere),
        "the running child's directory is the one its recipe named",
    );

    let stopped = manager.stop(McpPort::new(port));
    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "the child stops cleanly: {stopped:?}",
    );
    assert_eq!(
        manager.child_working_dir(),
        None,
        "a stopped child leaves no directory behind",
    );
}

#[test]
fn a_child_from_a_recipe_with_no_directory_reports_the_hosts_own() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager = HostManager::with_orphan_watch(
        Box::new(GatedStubSpawner::new(gate)),
        no_boot_deadline_config(),
        Box::new(WatchFreePort),
    );
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };

    let outcome = manager.launch(McpPort::new(port), &sample_spec());
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );
    assert_eq!(manager.child_working_dir(), Some(WorkingDir::new(here)));
    let stopped = manager.stop(McpPort::new(port));
    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "the child stops cleanly: {stopped:?}",
    );
}

#[test]
fn with_two_children_recorded_the_directory_is_the_last_one_launched() {
    let gates = gated_listeners(2);
    let ports: Vec<McpPort> = gates.iter().map(|(port, _)| *port).collect();
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(PortGatedSpawner::new(calls, gates)),
        always_spawning_no_boot_deadline_config(),
        Box::new(WatchFreePort),
    );
    let (Some(first), Some(second)) = (ports.first().copied(), ports.get(1).copied()) else {
        unreachable!("the test opened two gated listeners, got: {ports:?}");
    };
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    let (earlier_dir, later_dir) = (WorkingDir::new(here), WorkingDir::new(std::env::temp_dir()));
    assert_ne!(
        earlier_dir, later_dir,
        "the two recipes name two directories, or this proves nothing",
    );

    let earlier = manager.launch(first, &recipe_in(&earlier_dir));
    let later = manager.launch(second, &recipe_in(&later_dir));

    assert!(
        matches!(earlier, LaunchOutcome::Launched { .. }),
        "the first launch becomes ready: {earlier:?}",
    );
    assert!(
        matches!(later, LaunchOutcome::Launched { .. }),
        "the second launch becomes ready: {later:?}",
    );
    assert_eq!(
        manager.child_working_dir(),
        Some(later_dir),
        "with no instance named the directory is the last child launched, not the first",
    );
}

#[test]
fn each_recorded_instance_reports_the_directory_its_own_recipe_named() {
    let gates = gated_listeners(2);
    let ports: Vec<McpPort> = gates.iter().map(|(port, _)| *port).collect();
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(PortGatedSpawner::new(calls, gates)),
        always_spawning_no_boot_deadline_config(),
        Box::new(WatchFreePort),
    );
    let (Some(first), Some(second)) = (ports.first().copied(), ports.get(1).copied()) else {
        unreachable!("the test opened two gated listeners, got: {ports:?}");
    };
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    let (earlier_dir, later_dir) = (WorkingDir::new(here), WorkingDir::new(std::env::temp_dir()));
    assert_ne!(
        earlier_dir, later_dir,
        "the two recipes name two directories, or this proves nothing",
    );

    let earlier = manager.launch(first, &recipe_in(&earlier_dir));
    let later = manager.launch(second, &recipe_in(&later_dir));

    assert!(
        matches!(earlier, LaunchOutcome::Launched { .. }),
        "the first launch becomes ready: {earlier:?}",
    );
    assert!(
        matches!(later, LaunchOutcome::Launched { .. }),
        "the second launch becomes ready: {later:?}",
    );
    let recorded = manager.instances();
    let (Some(earlier_record), Some(later_record)) = (recorded.first(), recorded.get(1)) else {
        unreachable!("both launches are recorded, got: {recorded:?}");
    };
    assert_eq!(
        manager.instance_working_dir(earlier_record.id()),
        Some(earlier_dir),
        "the older instance answers with its own recipe's directory, not the last child's",
    );
    assert_eq!(
        manager.instance_working_dir(later_record.id()),
        Some(later_dir),
        "the newer instance answers with its own recipe's directory",
    );
}

use std::sync::{Arc, Mutex};

use gdtf_qa_mcp::{
    CargoPackage, EnvOverrides, FeatureList, HostLifecycle, HostManager, LaunchOutcome, LaunchSpec,
    QaChannel, QaPort, StopOutcome, WorkingDir,
};

use crate::{
    fake_child::{CallLog, PortGatedSpawner},
    support::{
        GatedStubSpawner, WatchFreePort, always_spawning_config, fast_config, gated_listeners,
        spawn_gated_fake_game,
    },
};

fn recipe_in(dir: &WorkingDir) -> LaunchSpec {
    LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
        FeatureList::default(),
        Some(dir.clone()),
        EnvOverrides::default(),
        QaChannel::game(),
    )
}

#[test]
fn a_running_child_reports_the_directory_its_recipe_named() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));
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

    let outcome = manager.launch(QaPort::new(port), &recipe_in(&elsewhere));
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );
    assert_eq!(
        manager.child_working_dir(),
        Some(elsewhere),
        "the running child's directory is the one its recipe named",
    );

    let stopped = manager.stop(QaPort::new(port));
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
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };

    let outcome = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );
    assert_eq!(manager.child_working_dir(), Some(WorkingDir::new(here)));
    let stopped = manager.stop(QaPort::new(port));
    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "the child stops cleanly: {stopped:?}",
    );
}

#[test]
fn with_two_children_recorded_the_directory_is_the_last_one_launched() {
    let gates = gated_listeners(2);
    let ports: Vec<QaPort> = gates.iter().map(|(port, _)| *port).collect();
    let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
    let mut manager = HostManager::with_orphan_watch(
        Box::new(PortGatedSpawner::new(calls, gates)),
        always_spawning_config(2000),
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

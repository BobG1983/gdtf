use gdtf_qa_mcp::{
    CargoPackage, EnvOverrides, FeatureList, HostLifecycle, HostManager, LaunchOutcome, LaunchSpec,
    QaChannel, QaPort, StopOutcome, WorkingDir,
};

use crate::support::{GatedStubSpawner, fast_config, spawn_gated_fake_game};

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

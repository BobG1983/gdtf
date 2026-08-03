use gdtf_qa_mcp::{
    CargoPackage, EnvOverrides, FeatureList, FeatureName, HostLifecycle, HostManager,
    LaunchFailure, LaunchOutcome, LaunchSpec, QaChannel, QaPort, StopOutcome, WorkingDir,
};

use crate::support::{
    GatedStubSpawner, STUB_STDERR_LINE, StubSpawner, fast_config, free_port, spawn_gated_fake_game,
};

#[test]
fn launch_becomes_ready_then_stops() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));

    let outcome = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    let LaunchOutcome::Launched {
        port: ready_port,
        pid,
    } = outcome
    else {
        unreachable!("launch against the fake server becomes ready: {outcome:?}");
    };
    assert_eq!(*ready_port, port);

    let stopped = manager.stop(QaPort::new(port));
    let StopOutcome::Stopped { pid: stopped_pid } = stopped else {
        unreachable!("stop reaps the running child: {stopped:?}");
    };
    assert_eq!(pid, stopped_pid, "stop reaps the child that launched");
    assert_eq!(
        manager.stop(QaPort::new(free_port())),
        StopOutcome::NotRunning
    );
}

fn recipe_with_features(features: &[&str]) -> LaunchSpec {
    LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
        FeatureList::new(
            features
                .iter()
                .map(|name| FeatureName::new((*name).to_owned()))
                .collect(),
        ),
        None,
        EnvOverrides::default(),
        QaChannel::game(),
    )
}

#[test]
fn second_launch_is_already_running() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));
    let spec = recipe_with_features(&["dynamic_linking", "net_qa", "dev_tools"]);

    let LaunchOutcome::Launched { pid: first_pid, .. } = manager.launch(QaPort::new(port), &spec)
    else {
        unreachable!("the first launch becomes ready");
    };
    let second = manager.launch(QaPort::new(port), &spec);
    let LaunchOutcome::AlreadyRunning {
        port: existing_port,
        pid,
        recipe,
    } = second
    else {
        unreachable!("a second launch is already-running, not a second spawn: {second:?}");
    };
    assert_eq!(*existing_port, port);
    assert_eq!(pid, first_pid, "already-running reports the same child");
    assert_eq!(
        recipe.features().render(),
        Some("dynamic_linking,net_qa,dev_tools".to_owned()),
        "already-running names the recipe the running child was launched from"
    );
    assert_ne!(
        *recipe,
        LaunchSpec::game_default(),
        "the reported recipe is the running child's, not the default"
    );

    let _ = manager.stop(QaPort::new(port));
}

#[test]
fn a_second_launch_of_a_different_recipe_is_rejected() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));
    let running_recipe = recipe_with_features(&["dynamic_linking", "net_qa"]);

    let LaunchOutcome::Launched { pid: first_pid, .. } =
        manager.launch(QaPort::new(port), &running_recipe)
    else {
        unreachable!("the first launch becomes ready");
    };
    let second = manager.launch(
        QaPort::new(port),
        &recipe_with_features(&["dynamic_linking", "net_qa", "dev_tools"]),
    );
    let LaunchOutcome::Failed(LaunchFailure::RecipeMismatch(running)) = second else {
        unreachable!("a different recipe is rejected, not reported as already running: {second:?}");
    };
    assert_eq!(*running, running_recipe, "the rejection names what is up");

    let StopOutcome::Stopped { pid } = manager.stop(QaPort::new(port)) else {
        unreachable!("the first child is still running after the rejection");
    };
    assert_eq!(pid, first_pid);
}

#[test]
fn an_unnamed_directory_matches_the_hosts_own_directory() {
    let (port, gate) = spawn_gated_fake_game();
    let mut manager =
        HostManager::with_config(Box::new(GatedStubSpawner::new(gate)), fast_config(2000));
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    let named = LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
        LaunchSpec::game_default().features().clone(),
        Some(WorkingDir::new(here)),
        EnvOverrides::default(),
        QaChannel::game(),
    );

    let LaunchOutcome::Launched { .. } = manager.launch(QaPort::new(port), &named) else {
        unreachable!("the first launch becomes ready");
    };
    let second = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    assert!(
        matches!(second, LaunchOutcome::AlreadyRunning { .. }),
        "the same checkout under two spellings is one recipe: {second:?}"
    );

    let _ = manager.stop(QaPort::new(port));
}

#[test]
fn launch_times_out_and_captures_stderr() {
    let port = free_port();
    let config = fast_config(800);
    let mut manager = HostManager::with_config(Box::new(StubSpawner), config);

    let outcome = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    let LaunchOutcome::Failed(LaunchFailure::Timeout { tail, waited }) = outcome else {
        unreachable!("no listener means the launch times out: {outcome:?}");
    };
    assert!(
        tail.contains(STUB_STDERR_LINE),
        "the failure carries the child's output tail: {}",
        tail.as_str()
    );
    assert_eq!(
        waited,
        config.boot_timeout(),
        "the failure reports the boot timeout the manager was configured with"
    );
    assert_eq!(manager.stop(QaPort::new(port)), StopOutcome::NotRunning);
}

#[test]
fn stop_with_nothing_running_is_not_running() {
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(2000));
    assert_eq!(
        manager.stop(QaPort::new(free_port())),
        StopOutcome::NotRunning
    );
}

//! What the REAL [`HostManager`] answers when asked where its child is running (GTW-923).
//!
//! This is the fact the render path reads to open a capture the child wrote at a relative
//! path. The manager is production code here — a real spawn, a real readiness probe against
//! the fake listener, a real stop — so the answer comes from the recipe the manager actually
//! retained, not from a fixture standing in for it.

use gdtf_qa_mcp::{
    CargoPackage, EnvOverrides, FeatureList, HostLifecycle, HostManager, LaunchOutcome, LaunchSpec,
    QaChannel, QaPort, StopOutcome, WorkingDir,
};

use crate::support::{StubSpawner, fast_config, spawn_fake_game};

/// A recipe that runs in `dir`.
fn recipe_in(dir: &WorkingDir) -> LaunchSpec {
    LaunchSpec::new(
        CargoPackage::new("grimdark_turfwar".to_owned()),
        FeatureList::default(),
        Some(dir.clone()),
        EnvOverrides::default(),
        QaChannel::game(),
    )
}

/// With a child running from a recipe that named its own directory, the manager reports THAT
/// directory — not the MCP host's. A `launch_*` naming a `working_dir` is exactly the GTW-875
/// case a capture could not be read back from.
#[test]
fn a_running_child_reports_the_directory_its_recipe_named() {
    let port = spawn_fake_game();
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(2000));
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

    let stopped = manager.stop();
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

/// A recipe that named NO directory still ran somewhere — the host's own current directory —
/// and the manager reports that rather than `None`. A capture from such a child is read
/// exactly where it always was, which is why the fix changes nothing for the same-checkout
/// case.
#[test]
fn a_child_from_a_recipe_with_no_directory_reports_the_hosts_own() {
    let port = spawn_fake_game();
    let mut manager = HostManager::with_config(Box::new(StubSpawner), fast_config(2000));
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };

    let outcome = manager.launch(QaPort::new(port), &LaunchSpec::game_default());
    assert!(
        matches!(outcome, LaunchOutcome::Launched { .. }),
        "launch against the fake server becomes ready: {outcome:?}",
    );
    assert_eq!(manager.child_working_dir(), Some(WorkingDir::new(here)));
    let stopped = manager.stop();
    assert!(
        matches!(stopped, StopOutcome::Stopped { .. }),
        "the child stops cleanly: {stopped:?}",
    );
}

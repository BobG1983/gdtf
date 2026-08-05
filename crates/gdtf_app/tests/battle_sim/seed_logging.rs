use std::{
    io::Write,
    process::{Command, Output},
    sync::{Arc, LazyLock, Mutex},
};

use bevy::log::tracing_subscriber::{self, fmt::MakeWriter, util::SubscriberInitExt as _};
use gdtf_battle_sim::rng::{BattleSeed, ShotRng};

use super::harness::*;

const SEED_ENV_VAR: &str = "GDTF_BATTLE_SEED";
const CASE_ENV_VAR: &str = "GDTF_SEED_LOGGING_CASE";
const PINNED_CASE: &str = "env-pinned";
const UNSET_CASE: &str = "wall-clock";
const PINNED: u64 = 0x0BAD_F00D_DEAD_BEEF;
// Libtest path of the test below, so a child runs that one case and nothing else.
const TEST_PATH: &str = "seed_logging::resolve_root_seed_drives_streams_and_logs";
// Printed by a case that ran to its end; its absence means the child ran no case.
const CASE_PASSED: &str = "gdtf-seed-logging-case-passed";

static LOG_CAPTURE: LazyLock<Arc<Mutex<Vec<u8>>>> = LazyLock::new(|| {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let _installed = tracing_subscriber::fmt()
        .with_writer(CaptureWriter(Arc::clone(&buf)))
        .with_ansi(false)
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .finish()
        .try_init();
    buf
});

#[derive(Clone)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureSink;

    fn make_writer(&'a self) -> Self::Writer {
        CaptureSink(Arc::clone(&self.0))
    }
}

struct CaptureSink(Arc<Mutex<Vec<u8>>>);

impl Write for CaptureSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut guard) = self.0.lock() {
            guard.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn captured_log() -> String {
    LOG_CAPTURE
        .lock()
        .map(|guard| String::from_utf8_lossy(&guard).into_owned())
        .unwrap_or_default()
}

// A child process is the only way to pin GDTF_BATTLE_SEED without mutating this
// process's environment, and it also makes the unset case independent of whatever
// the developer has exported.
fn spawn_case(case: &str) -> std::io::Result<Output> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg("--nocapture")
        .arg(TEST_PATH)
        .env(CASE_ENV_VAR, case);
    if case == PINNED_CASE {
        command.env(SEED_ENV_VAR, PINNED.to_string());
    } else {
        command.env_remove(SEED_ENV_VAR);
    }
    command.output()
}

fn run_case(case: &str) {
    let spawned = spawn_case(case);
    assert!(
        spawned.is_ok(),
        "the `{case}` case must re-run in a child process; spawning failed: {:?}",
        spawned.as_ref().err(),
    );
    let Ok(output) = spawned else {
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "the `{case}` case failed in its child process ({}).\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status,
    );
    assert!(
        stdout.contains(CASE_PASSED),
        "the `{case}` child ran no case — `{TEST_PATH}` matched no test. stdout:\n{stdout}",
    );
}

fn assert_env_pinned_seed_drives_streams() {
    LazyLock::force(&LOG_CAPTURE);

    let mut app = walk_app(None);
    let reached = drive_to_generation(&mut app);
    assert!(
        reached,
        "the env-pinned walk should reach Generation within {BUDGET} updates; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "the env-pinned Generation must insert a ShotRng stream (resolve_root_seed path)",
    );

    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut expected = ShotRng::from_root(BattleSeed::new(PINNED));
    assert_eq!(
        world_first,
        expected.next_u64(),
        "the resolved ShotRng's first draw must equal \
         ShotRng::from_root(BattleSeed::new(PINNED)).next_u64() — proving GDTF_BATTLE_SEED drove \
         the streams end-to-end through resolve_root_seed (not the with_seed override)",
    );
    println!("{CASE_PASSED}");
}

fn assert_unset_seed_is_wall_clock_and_logged() {
    LazyLock::force(&LOG_CAPTURE);

    let mut app = walk_app(None);
    assert!(
        drive_to_generation(&mut app),
        "the unset (wall-clock) walk should reach Generation within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut zero_stream = ShotRng::from_root(BattleSeed::new(0));
    assert_ne!(
        world_first,
        zero_stream.next_u64(),
        "the unset path must still derive a NON-ZERO wall-clock seed (its stream must differ from \
         the zero-seed stream)",
    );
    let logs = captured_log();
    assert!(
        logs.contains("resolved BattleSeed"),
        "battle setup must log the resolved BattleSeed (the replay handle) at info!; the captured \
         log did not contain the expected line. Captured:\n{logs}",
    );
    println!("{CASE_PASSED}");
}

#[test]
fn resolve_root_seed_drives_streams_and_logs() {
    match std::env::var(CASE_ENV_VAR).ok().as_deref() {
        Some(PINNED_CASE) => assert_env_pinned_seed_drives_streams(),
        Some(UNSET_CASE) => assert_unset_seed_is_wall_clock_and_logged(),
        _ => {
            run_case(PINNED_CASE);
            run_case(UNSET_CASE);
        }
    }
}

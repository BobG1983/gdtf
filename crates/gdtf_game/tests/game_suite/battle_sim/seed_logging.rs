use std::{
    io::Write,
    process::{Command, Output},
    sync::{Arc, LazyLock, Mutex},
};

use bevy::log::tracing_subscriber::{self, fmt::MakeWriter, util::SubscriberInitExt as _};
use gdtf_battle_sim::rng::{BattleSeed, ShotRng};
use gdtf_game::test_support::BattleAppBuilder;

use super::harness::*;

const PINNED: u64 = 0x0BAD_F00D_DEAD_BEEF;
// Libtest path of the ignored case below, so a child runs that one and nothing else.
const CHILD_TEST_PATH: &str =
    "battle_sim::seed_logging::an_unpinned_battle_seeds_from_the_wall_clock_and_logs_it";
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

// LOG_CAPTURE wins the try_init race only when the case runs alone, so libtest runs it in a child.
fn spawn_child_case() -> std::io::Result<Output> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg("--ignored")
        .arg("--nocapture")
        .arg(CHILD_TEST_PATH);
    command.output()
}

#[test]
fn a_pinned_battle_seed_drives_the_shot_stream() {
    let mut app = BattleAppBuilder::new()
        .with_seed(BattleSeed::new(PINNED))
        .build();

    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "a battle set up from a pinned seed must insert a ShotRng stream",
    );
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut expected = ShotRng::from_root(BattleSeed::new(PINNED));
    assert_eq!(
        world_first,
        expected.next_u64(),
        "the resolved ShotRng's first draw must equal \
         ShotRng::from_root(BattleSeed::new(PINNED)).next_u64(). That is what proves the pinned \
         seed drove the streams end to end through the seed override",
    );
}

#[test]
#[ignore = "installs a global log subscriber, so it runs alone in a child process"]
fn an_unpinned_battle_seeds_from_the_wall_clock_and_logs_it() {
    LazyLock::force(&LOG_CAPTURE);

    let mut app = walk_app(None);
    drive_to_generation(&mut app);
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut zero_stream = ShotRng::from_root(BattleSeed::new(0));
    assert_ne!(
        world_first,
        zero_stream.next_u64(),
        "the unpinned path must still derive a NON-ZERO wall-clock seed (its stream must differ \
         from the zero-seed stream)",
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
fn the_wall_clock_case_passes_in_its_own_process() {
    let spawned = spawn_child_case();
    assert!(
        spawned.is_ok(),
        "the wall-clock case must re-run in a child process; spawning failed: {:?}",
        spawned.as_ref().err(),
    );
    let Ok(output) = spawned else {
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "the wall-clock case failed in its child process ({}).\nstdout:\n{stdout}\nstderr:\n\
         {stderr}",
        output.status,
    );
    assert!(
        stdout.contains(CASE_PASSED),
        "the child ran no case. `{CHILD_TEST_PATH}` matched no ignored test. stdout:\n{stdout}",
    );
}

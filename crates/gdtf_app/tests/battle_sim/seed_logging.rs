use std::{
    io::Write,
    sync::{Arc, LazyLock, Mutex},
};

use bevy::log::tracing_subscriber::{self, fmt::MakeWriter, util::SubscriberInitExt as _};
use gdtf_battle_sim::rng::{BattleSeed, ShotRng};

use super::harness::*;


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

#[allow(
    unsafe_code,
    reason = "env-var test must mutate the process environment"
)]
fn set_seed_env(key: &str, value: &str) {
    // SAFETY: serialized, single `#[test]` use; the var is removed right after the
    unsafe { std::env::set_var(key, value) };
}

#[allow(
    unsafe_code,
    reason = "env-var test must mutate the process environment"
)]
fn clear_seed_env(key: &str) {
    // SAFETY: serialized, single `#[test]` use; restores the process environment.
    unsafe { std::env::remove_var(key) };
}

#[test]
fn resolve_root_seed_drives_streams_and_logs() {
    const SEED_ENV_VAR: &str = "GDTF_BATTLE_SEED";
    const PINNED: u64 = 0x0BAD_F00D_DEAD_BEEF;

    LazyLock::force(&LOG_CAPTURE);

    set_seed_env(SEED_ENV_VAR, &PINNED.to_string());
    let mut app = walk_app(None); 
    let reached = drive_to_generation(&mut app);
    clear_seed_env(SEED_ENV_VAR);
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
}

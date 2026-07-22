//! GTW-14/C7: composition-root seed resolution end-to-end with log capture.

use std::{
    io::Write,
    sync::{Arc, LazyLock, Mutex},
};

use bevy::log::tracing_subscriber::{self, fmt::MakeWriter, util::SubscriberInitExt as _};
use gdtf_battle_sim::rng::{BattleSeed, ShotRng};

use super::harness::*;

// ---------------------------------------------------------------------------
// GTW-14 / C7 (M3): end-to-end coverage of the composition-root seed resolution.
//
// Every other app test pre-injects a `BattleSeed` via `with_seed`, so
// `request_battle_setup` takes the override branch and BYPASSES `resolve_root_seed`
// entirely. These tests drive the REAL resolution path: build WITHOUT a seed override
// so `request_battle_setup` calls `resolve_root_seed`, which reads `GDTF_BATTLE_SEED`
// (or falls back to wall-clock) and the unconditional setup `info!` (m2) fires.
// ---------------------------------------------------------------------------

/// Process-wide log capture, installed ONCE for this test binary. Bevy's
/// `MinimalPlugins` harness installs no `LogPlugin`, so this is the only global
/// tracing subscriber; it records every event into a shared buffer so the
/// seed-resolution test can prove the unconditional setup `info!` (m2) fires.
static LOG_CAPTURE: LazyLock<Arc<Mutex<Vec<u8>>>> = LazyLock::new(|| {
    let buf = Arc::new(Mutex::new(Vec::new()));
    // Install once for the binary; a later/competing subscriber makes this a no-op
    // that `try_init` reports as `Err` — harmless, the bound value is just dropped.
    let _installed = tracing_subscriber::fmt()
        .with_writer(CaptureWriter(Arc::clone(&buf)))
        .with_ansi(false)
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .finish()
        .try_init();
    buf
});

/// A `MakeWriter` that appends all formatted log output into a shared byte buffer.
#[derive(Clone)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureSink;

    fn make_writer(&'a self) -> Self::Writer {
        CaptureSink(Arc::clone(&self.0))
    }
}

/// The per-write sink handed out by [`CaptureWriter`]; pushes bytes into the buffer.
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

/// Snapshot the captured log text so far.
fn captured_log() -> String {
    LOG_CAPTURE
        .lock()
        .map(|guard| String::from_utf8_lossy(&guard).into_owned())
        .unwrap_or_default()
}

/// Set the `GDTF_BATTLE_SEED` env var for the duration of the env-pinned case.
///
/// `std::env::set_var` is `unsafe` in edition 2024 because the process environment
/// is global and not thread-safe; the single serialized test below restores the var
/// immediately after the pinned drive (no other observer reads it in between).
#[allow(
    unsafe_code,
    reason = "env-var test must mutate the process environment"
)]
fn set_seed_env(key: &str, value: &str) {
    // SAFETY: serialized, single `#[test]` use; the var is removed right after the
    // env-pinned drive (see `clear_seed_env`), so no concurrent reader observes it.
    unsafe { std::env::set_var(key, value) };
}

/// Remove the `GDTF_BATTLE_SEED` env var (see [`set_seed_env`]).
#[allow(
    unsafe_code,
    reason = "env-var test must mutate the process environment"
)]
fn clear_seed_env(key: &str) {
    // SAFETY: serialized, single `#[test]` use; restores the process environment.
    unsafe { std::env::remove_var(key) };
}

/// GTW-14 / C7 (M3) — `resolve_root_seed` drives the streams end-to-end and the
/// resolved seed is logged. BOTH cases run in ONE serialized test because
/// `GDTF_BATTLE_SEED` is process-global; the var is restored between them.
///
/// (a) **env-pinned**: with `GDTF_BATTLE_SEED` set and NO `with_seed` override, the
///     world's resolved `ShotRng` first draw equals
///     `ShotRng::from_root(BattleSeed::new(<that value>))` — proving the env seed
///     actually drove the per-subsystem streams (not the override path).
/// (b) **unset**: a NON-ZERO (wall-clock) seed is still produced — the resolved
///     stream differs from the zero-seed stream — and the unconditional replay-handle
///     `info!` (m2) fired at battle setup (captured via [`LOG_CAPTURE`]).
#[test]
fn resolve_root_seed_drives_streams_and_logs() {
    // Mirrors the (private) production const in `…/battle_sim/seed.rs`.
    const SEED_ENV_VAR: &str = "GDTF_BATTLE_SEED";
    // A fixed, distinctive seed `resolve_root_seed` must parse and thread through.
    const PINNED: u64 = 0x0BAD_F00D_DEAD_BEEF;

    // Force the global log-capture subscriber up before any app drives, so the
    // unconditional setup `info!` is recorded for the case-(b) assertion below.
    LazyLock::force(&LOG_CAPTURE);

    // --- (a) env-pinned: GDTF_BATTLE_SEED drives the actual streams ---
    set_seed_env(SEED_ENV_VAR, &PINNED.to_string());
    let mut app = walk_app(None); // NO with_seed → request_battle_setup calls resolve_root_seed
    let reached = drive_to_generation(&mut app);
    // Restore the global env var immediately — before any assert and before case (b).
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

    // The world's resolved ShotRng (untouched — no fire act ran) must be identical
    // to a fresh stream derived from the pinned env seed: GDTF_BATTLE_SEED drove it.
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut expected = ShotRng::from_root(BattleSeed::new(PINNED));
    assert_eq!(
        world_first,
        expected.next_u64(),
        "the resolved ShotRng's first draw must equal \
         ShotRng::from_root(BattleSeed::new(PINNED)).next_u64() — proving GDTF_BATTLE_SEED drove \
         the streams end-to-end through resolve_root_seed (not the with_seed override)",
    );

    // --- (b) unset: a non-zero (wall-clock) seed is still produced AND logged ---
    let mut app = walk_app(None);
    assert!(
        drive_to_generation(&mut app),
        "the unset (wall-clock) walk should reach Generation within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    // A non-zero, time-derived seed was produced: its stream is NOT the zero-seed
    // stream (the wall-clock seed is microseconds-since-epoch, never 0).
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut zero_stream = ShotRng::from_root(BattleSeed::new(0));
    assert_ne!(
        world_first,
        zero_stream.next_u64(),
        "the unset path must still derive a NON-ZERO wall-clock seed (its stream must differ from \
         the zero-seed stream)",
    );
    // And the unconditional replay-handle info! (m2) fired at battle setup.
    let logs = captured_log();
    assert!(
        logs.contains("resolved BattleSeed"),
        "battle setup must log the resolved BattleSeed (the replay handle) at info!; the captured \
         log did not contain the expected line. Captured:\n{logs}",
    );
}

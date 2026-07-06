//! Unit tests for the DEV-ONLY screenshot / fire-trigger config + the headless
//! fire-trigger path (GTW-297, extended GTW-306, made loud + suite-visible GTW-590).
//!
//! The config tests cover the pure parse cores: the capture-path gate, the
//! [`CaptureFrame`] / [`CaptureFrames`] / [`FireAtFrame`] parse, and the multi-frame
//! [`frame_path`] naming. The `resolve` tests pin the env-snapshot resolution — the
//! pinned QA env set to its exact frame schedule, plus every GTW-590 loud diagnostic
//! and its rendered `warn!` text. The `loudness` tests pin the EMISSION of those loud
//! lines on the real paths (the config warn loop, the `build()` output-dir `error!`,
//! a trigger skip `warn!`) under the shared log capture. The `schedule` tests pin the
//! registration / state-gating layer: the REAL plugin fires a capture request on
//! exactly the scheduled `BattleRunning` frames of the real state chain. All drive
//! the REAL pure cores with INJECTED values rather than mutating the process-global
//! env vars (deterministic under parallel tests) — the same code `from_env` runs.
//!
//! The ACTUAL GPU readback / PNG encode needs a real render device, so it is NOT
//! headless-testable — it is verified by RUNNING the app (the GTW-590 C5 in-engine
//! evidence). The fire trigger, by contrast, IS headless: it only writes a
//! `FireRequested` message, so the
//! [`trigger_fire_at_frame_emits_on_the_real_path`] test drives the REAL system on a
//! minimal app and asserts exactly one `FireRequested` at frame N.

mod config;
mod fall;
mod fire;
mod loudness;
mod resolve;
mod schedule;

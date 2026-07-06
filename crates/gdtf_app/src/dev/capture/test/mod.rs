//! Unit tests for the DEV-ONLY screenshot / capture config (GTW-297, extended
//! GTW-306, made loud + suite-visible GTW-590).
//!
//! The config tests cover the pure parse cores: the capture-path gate, the
//! [`CaptureFrame`] / [`CaptureFrames`] parse, and the multi-frame [`frame_path`]
//! naming. The `resolve` tests pin the env-snapshot resolution — the pinned QA env
//! set to its exact frame schedule, plus every GTW-590 loud diagnostic and its
//! rendered `warn!` text. The `loudness` tests pin the EMISSION of those loud lines
//! on the real paths (the config warn loop, the `build()` output-dir `error!`, a
//! trigger skip `warn!`) under the shared log capture. The `schedule` tests pin the
//! registration / state-gating layer: the REAL plugin fires a capture request on
//! exactly the scheduled `BattleRunning` frames of the real state chain. All drive
//! the REAL pure cores with INJECTED values rather than mutating the process-global
//! env vars (deterministic under parallel tests) — the same code `from_env` runs.
//!
//! The ACTUAL GPU readback / PNG encode needs a real render device, so it is NOT
//! headless-testable — it is verified by RUNNING the app (the GTW-590 C5 in-engine
//! evidence). The fire / fall TRIGGER tests (parse gates + headless drives of the
//! real paths) live with their systems in `crate::dev::drive` (GTW-632).

mod config;
mod loudness;
mod resolve;
mod schedule;

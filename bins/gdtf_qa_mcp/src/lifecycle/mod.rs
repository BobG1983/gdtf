//! The game-lifecycle layer — launching and stopping the game child (GTW-745).
//!
//! Backs the `launch_game` / `stop_game` MCP tools. Everything is blocking `std` —
//! [`std::process::Command`] + [`std::thread`], no tokio. The pieces:
//!
//! - [`values`] — the small newtypes the layer talks in (pid, stderr tail, timing knobs)
//!   plus the [`ChildStatus`] / [`Readiness`] status answers.
//! - [`config`] — the [`LifecycleConfig`] timing knobs.
//! - [`outcome`] — the typed [`LaunchOutcome`] / [`StopOutcome`] results.
//! - [`child`] — the [`GameChild`] trait and its real [`ProcessChild`] (stderr capture +
//!   process-group signalling).
//! - [`launch`] — the [`LaunchSpec`] recipe (package, features, working directory,
//!   environment overrides) one launch runs, and the newtypes it is written in.
//! - [`spawn`] — the [`GameSpawner`] trait and its real [`CargoSpawner`], which turns a
//!   recipe into a `cargo run` command.
//! - [`probe`] — the readiness [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello)
//!   round-trip.
//! - [`manager`] — the [`GameManager`] that owns the one running child and drives launch /
//!   stop, plus the [`GameLifecycle`] trait the tool layer calls.

pub mod child;
pub mod config;
pub mod launch;
pub mod manager;
pub mod outcome;
pub mod probe;
pub mod spawn;
pub mod values;

pub use child::{GameChild, ProcessChild};
pub use config::LifecycleConfig;
pub use launch::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    LaunchSpec, WorkingDir,
};
pub use manager::{GameLifecycle, GameManager};
pub use outcome::{LaunchFailure, LaunchOutcome, StopOutcome};
pub use spawn::{CargoSpawner, GameSpawner, build_command};
pub use values::{
    BootTimeout, ChildPid, ChildStatus, KillGrace, PollInterval, ProbeTimeout, Readiness,
    SpawnError, StderrTail,
};

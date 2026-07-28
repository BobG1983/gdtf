//! The child-lifecycle layer — launching and stopping a managed child (GTW-745; made
//! host-neutral in GTW-808).
//!
//! Backs the `launch_game` / `stop_game` and `launch_editor` / `stop_editor` MCP tools:
//! one [`HostManager`] per host, each owning at most one child. Everything is blocking `std` —
//! [`std::process::Command`] + [`std::thread`], no tokio. The pieces:
//!
//! - [`values`] — the small newtypes the layer talks in (pid, stderr tail, timing knobs)
//!   plus the [`ChildStatus`] / [`Readiness`] status answers.
//! - [`config`] — the [`LifecycleConfig`] timing knobs.
//! - [`outcome`] — the typed [`LaunchOutcome`] / [`StopOutcome`] results.
//! - [`child`] — the [`ManagedChild`] trait and its real [`ProcessChild`] (stderr capture +
//!   process-group signalling).
//! - [`launch`] — the [`LaunchSpec`] recipe (package, features, working directory,
//!   environment overrides, and the [`QaChannel`] variable names) one launch runs, and
//!   the newtypes it is written in.
//! - [`spawn`] — the [`ChildSpawner`] trait and its real [`CargoSpawner`], which turns a
//!   recipe into a `cargo run` command.
//! - [`probe`] — the readiness [`Hello`](gdtf_qa_protocol::envelope::QaRequest::Hello)
//!   round-trip.
//! - [`manager`] — the [`HostManager`] that owns one running child and drives launch /
//!   stop, plus the [`HostLifecycle`] trait the tool layer calls.

pub mod child;
pub mod config;
pub mod launch;
pub mod manager;
pub mod outcome;
pub mod probe;
pub mod spawn;
pub mod values;

pub use child::{ManagedChild, ProcessChild};
pub use config::LifecycleConfig;
pub use launch::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    LaunchSpec, QaChannel, WorkingDir,
};
pub use manager::{HostLifecycle, HostManager};
pub use outcome::{LaunchFailure, LaunchOutcome, StopOutcome};
pub use spawn::{CargoSpawner, ChildSpawner, build_command};
pub use values::{
    BootTimeout, ChildPid, ChildStatus, KillGrace, PollInterval, ProbeTimeout, Readiness,
    SpawnError, StderrTail,
};

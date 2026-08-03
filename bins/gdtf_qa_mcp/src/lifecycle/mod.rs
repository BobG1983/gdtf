//! Launch, stop, and orphan handling for host processes.

pub mod child;
pub mod config;
pub mod launch;
pub mod manager;
pub mod orphan;
pub mod outcome;
pub mod probe;
pub mod spawn;
pub mod values;

pub use child::{ManagedChild, OUTPUT_TAIL_LINES, ProcessChild};
pub use config::LifecycleConfig;
pub use launch::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    LaunchSpec, QaChannel, WorkingDir,
};
pub use manager::{HostLifecycle, HostManager};
pub use orphan::{OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, PortHold, SystemOrphanWatch};
pub use outcome::{LaunchFailure, LaunchOutcome, StopOutcome};
pub use spawn::{CargoSpawner, ChildSpawner, build_command};
pub use values::{
    BootTimeout, ChildPid, ChildStatus, FailureTail, KillGrace, OutputTail, PollInterval,
    ProbeTimeout, Readiness, SpawnError, TailLines,
};

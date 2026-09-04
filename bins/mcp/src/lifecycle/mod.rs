//! Launch, stop, and orphan handling for host processes.

pub mod child;
pub mod config;
pub mod launch;
pub mod liveness;
pub mod manager;
pub mod orphan;
pub mod outcome;
pub mod probe;
pub mod spawn;
pub mod sweep;
pub mod values;

pub use child::{ManagedChild, OUTPUT_TAIL_LINES, ProcessChild};
pub use config::{LaunchPolicy, LifecycleConfig};
pub use launch::{
    CargoPackage, CargoProfile, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList,
    FeatureName, LaunchSpec, QaChannel, WorkingDir,
};
pub use liveness::{ChildLiveness, SystemLiveness};
pub use manager::{HostLifecycle, HostManager};
pub use orphan::{OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, PortHold, SystemOrphanWatch};
pub use outcome::{LaunchFailure, LaunchOutcome, StopOutcome};
pub use spawn::{CargoSpawner, ChildSpawner, build_command};
pub use sweep::{SweepClock, SweepDue, SweepEntry, SweepSchedule};
pub use values::{
    BootTimeout, ChildPid, ChildStatus, FailureTail, InstanceId, KillGrace, OutputTail,
    PollInterval, ProbeTimeout, Readiness, RecordedInstance, SpawnError, SweepInterval, TailLines,
};

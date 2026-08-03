pub mod base64;
pub mod error;
pub mod hosts;
pub mod lifecycle;
pub mod link;
pub mod mcp;
pub mod rpc;
pub mod serve;

pub use error::McpError;
pub use hosts::{HostPair, HostSet, QaHost};
pub use lifecycle::{
    BootTimeout, CargoPackage, CargoSpawner, ChildPid, ChildSpawner, EnvOverrides, EnvVar,
    EnvVarName, EnvVarValue, FailureTail, FeatureList, FeatureName, HostLifecycle, HostManager,
    KillGrace, LaunchFailure, LaunchOutcome, LaunchSpec, LifecycleConfig, ManagedChild,
    OUTPUT_TAIL_LINES, OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, OutputTail, PollInterval,
    PortHold, ProbeTimeout, ProcessChild, QaChannel, StopOutcome, SystemOrphanWatch, TailLines,
    WorkingDir, build_command,
};
pub use link::{LINK_TIMEOUT, LinkTimeout, QaClient, QaLink, QaPort};
pub use rpc::dispatch;
pub use serve::run_stdio;

//! MCP bridge that launches game/editor hosts and forwards QA tool calls over net QA.

/// Standard base64 encoding for tool payloads.
pub mod base64;
/// Bridge error types.
pub mod error;
/// Game and editor host handles.
pub mod hosts;
/// Child process lifecycle (launch, stop, orphans).
pub mod lifecycle;
/// TCP link to a host's net QA channel.
pub mod link;
/// MCP protocol handlers and tool schemas.
pub mod mcp;
/// JSON-RPC dispatch over stdio.
pub mod rpc;
/// Stdio serve loop.
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

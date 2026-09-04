//! MCP bridge that launches registered hosts and forwards QA tool calls over MCP.

/// Standard base64 encoding for tool payloads.
pub mod base64;
/// Bridge error types.
pub mod error;
/// Registered hosts and their handles.
pub mod hosts;
/// Child process lifecycle (launch, stop, orphans).
pub mod lifecycle;
/// TCP link to a host's MCP channel.
pub mod link;
/// MCP protocol handlers and tool schemas.
pub mod mcp;
/// JSON-RPC dispatch over stdio.
pub mod rpc;
/// Stdio serve loop.
pub mod serve;

pub use error::McpError;
pub use hosts::{HostName, HostPair, HostRegistry, HostRuntime, HostSet, QaHostSpec};
pub use lifecycle::{
    BootTimeout, CargoPackage, CargoProfile, CargoSpawner, ChildLiveness, ChildPid, ChildSpawner,
    EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FailureTail, FeatureList, FeatureName,
    HostLifecycle, HostManager, InstanceId, KillGrace, LaunchFailure, LaunchOutcome, LaunchPolicy,
    LaunchSpec, LifecycleConfig, ManagedChild, OUTPUT_TAIL_LINES, OrphanPid, OrphanStop,
    OrphanTarget, OrphanWatch, OutputTail, PollInterval, PortHold, ProbeTimeout, ProcessChild,
    QaChannel, RecordedInstance, StopOutcome, SweepClock, SweepDue, SweepEntry, SweepInterval,
    SweepSchedule, SystemLiveness, SystemOrphanWatch, TailLines, WorkingDir, build_command,
};
pub use link::{LINK_TIMEOUT, LinkTimeout, QaClient, QaLink, QaPort};
pub use mcp::{
    ServerIdentity, ServerName, ServerVersion, handle_tool_call, initialize_result,
    resolve_instance, tools_list_result,
};
pub use rpc::dispatch;
pub use serve::run_stdio;

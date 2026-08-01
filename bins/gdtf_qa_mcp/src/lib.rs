//! `gdtf_qa_mcp` — the MCP stdio host that bridges a language-model harness to the
//! running game's and content editor's `net_qa` control channels (GTW-741, QA-net T8;
//! extended to the editor by GTW-808).
//!
//! It speaks a small, hand-rolled JSON-RPC 2.0 subset on stdin/stdout (newline-delimited,
//! the MCP stdio convention) and, per `tools/call`, either drives a child process directly or
//! carries a request to it over a loopback [`TcpStream`](std::net::TcpStream) using the
//! shared [`gdtf_qa_protocol`] framing. It pulls in NO Bevy, NO tokio, NO rmcp — everything
//! is blocking `std` I/O.
//!
//! # Layers
//!
//! - [`serve`] — the blocking stdin/stdout loop.
//! - [`rpc`] — JSON-RPC 2.0 envelope building + method dispatch.
//! - [`mcp`] — the MCP method semantics: `initialize`, the tool registry, `tools/call`.
//! - [`hosts`] — the [`QaHost`] pair (game, editor) and the [`HostSet`] that resolves a
//!   call to its host's link + lifecycle.
//! - [`link`] — the [`QaLink`] to a running child and its real [`QaClient`].
//! - [`lifecycle`] — starting and stopping a child process, and capturing what it prints
//!   (`std::process::Command` + threads, no tokio).
//! - [`base64`] / [`error`] — the image encoder an attachment rides on, and the link error
//!   vocabulary.
//!
//! # Five tools, and none of them names a command
//!
//! `launch`, `stop`, `logs`, `commands`, `run` — each taking the same `host` argument
//! (`"game"` or `"editor"`), so which child a call reaches is the CALL's to say. The first
//! three drive a child process through its [`lifecycle::HostManager`]; the last two carry a
//! [`QaRequest`](gdtf_qa_protocol::message::QaRequest) to a running one.
//!
//! What a host can DO is not in this binary at all. `commands` reads that host's live
//! catalogue and `run` calls one entry by name, so a host gaining a command changes nothing
//! here — no new tool, no rebuild of this binary, and no MCP reconnect. Start a session with
//! `commands`: it is the truth about what the build in front of you offers.

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

//! `gdtf_qa_mcp` — the MCP stdio host that bridges a language-model harness to the
//! running game's and content editor's `net_qa` control channels (GTW-741, QA-net T8;
//! extended to the editor by GTW-808).
//!
//! It speaks a small, hand-rolled JSON-RPC 2.0 subset on stdin/stdout (newline-delimited,
//! the MCP stdio convention) and, per `tools/call`, forwards to the child over a loopback
//! [`TcpStream`](std::net::TcpStream) using the shared [`gdtf_qa_protocol`] framing. It
//! pulls in NO Bevy, NO tokio, NO rmcp — everything is blocking `std` I/O.
//!
//! # Layers
//!
//! - [`serve`] — the blocking stdin/stdout loop.
//! - [`rpc`] — JSON-RPC 2.0 envelope building + method dispatch.
//! - [`mcp`] — the MCP method semantics: `initialize`, the tool registry, `tools/call`.
//! - [`hosts`] — the [`QaHost`] pair (game, editor) and the [`HostSet`] that resolves a
//!   tool to its host's link + lifecycle.
//! - [`link`] — the [`QaLink`] to a running child and its real [`QaClient`].
//! - [`lifecycle`] — starting and stopping a child process for the four launch / stop
//!   tools (`std::process::Command` + threads, no tokio).
//! - [`base64`] / [`error`] — the image-content encoder and the link error vocabulary.
//!
//! # Tools
//!
//! Twelve forwarding tools map 1:1 onto
//! [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest)s: ten against the GAME —
//! `send_input`, `query_state`, `get_output`, `take_screenshot`, `screenshot_after`
//! (GTW-749), `app_flow`, `start_battle` (GTW-760), `stepper_control`,
//! `activate_menu_item`, `focus_control` — and two against the CONTENT EDITOR,
//! `get_editor_query_options` and `query_editor` (GTW-805's query pair, reachable from a
//! client since GTW-808). The [`mcp::tools`] registry names each request it forwards and
//! which host it forwards to. Four host-local tools — `launch_game` / `stop_game` and
//! `launch_editor` / `stop_editor` — start and stop the two child processes themselves
//! through their [`lifecycle::HostManager`]s (GTW-745, GTW-808).
//!
//! `start_battle` is what carries a cold-launched game from the menu into a battle, so the
//! battle-only tools (`query_state`, `send_input`, `get_output`) become available at all.
//! `get_editor_query_options` is the editor's counterpart poll: it reports the editor's
//! `Load` / `Editing` readiness and the topics it will answer right now.

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
    EnvVarName, EnvVarValue, FeatureList, FeatureName, HostLifecycle, HostManager, KillGrace,
    LaunchFailure, LaunchOutcome, LaunchSpec, LifecycleConfig, ManagedChild, OrphanPid, OrphanStop,
    OrphanTarget, OrphanWatch, PollInterval, PortHold, ProbeTimeout, ProcessChild, QaChannel,
    StderrTail, StopOutcome, SystemOrphanWatch, WorkingDir, build_command,
};
pub use link::{LINK_TIMEOUT, LinkTimeout, QaClient, QaLink, QaPort};
pub use rpc::dispatch;
pub use serve::run_stdio;

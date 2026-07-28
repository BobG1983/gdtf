//! `gdtf_qa_mcp` — the MCP stdio host that bridges a language-model harness to the
//! running game's `net_qa` control channel (GTW-741, QA-net T8).
//!
//! It speaks a small, hand-rolled JSON-RPC 2.0 subset on stdin/stdout (newline-delimited,
//! the MCP stdio convention) and, per `tools/call`, forwards to the game over a loopback
//! [`TcpStream`](std::net::TcpStream) using the shared [`gdtf_qa_protocol`] framing. It
//! pulls in NO Bevy, NO tokio, NO rmcp — everything is blocking `std` I/O.
//!
//! # Layers
//!
//! - [`serve`] — the blocking stdin/stdout loop.
//! - [`rpc`] — JSON-RPC 2.0 envelope building + method dispatch.
//! - [`mcp`] — the MCP method semantics: `initialize`, the tool registry, `tools/call`.
//! - [`game`] — the [`GameLink`] to the running game and its real [`GameClient`].
//! - [`lifecycle`] — starting and stopping the game process for the `launch_game` /
//!   `stop_game` tools (`std::process::Command` + threads, no tokio).
//! - [`base64`] / [`error`] — the image-content encoder and the link error vocabulary.
//!
//! # Tools
//!
//! Seven forwarding tools map 1:1 onto
//! [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest)s: `send_input`, `query_state`,
//! `get_output`, `take_screenshot`, `screenshot_after` (GTW-749), `app_flow`, and
//! `start_battle` (GTW-760) — the [`mcp::tools`] registry names each request it forwards.
//! Two host-local tools — `launch_game` and `stop_game` — start and stop the game process
//! itself through the [`lifecycle::GameManager`] (GTW-745).
//!
//! `start_battle` is what carries a cold-launched game from the menu into a battle, so the
//! battle-only tools (`query_state`, `send_input`, `get_output`) become available at all.

pub mod base64;
pub mod error;
pub mod game;
pub mod lifecycle;
pub mod mcp;
pub mod rpc;
pub mod serve;

pub use error::McpError;
pub use game::{GameClient, GameLink, GamePort};
pub use lifecycle::{
    BootTimeout, CargoPackage, CargoSpawner, ChildPid, EnvOverrides, EnvVar, EnvVarName,
    EnvVarValue, FeatureList, FeatureName, GameChild, GameLifecycle, GameManager, GameSpawner,
    KillGrace, LaunchFailure, LaunchOutcome, LaunchSpec, LifecycleConfig, PollInterval,
    ProbeTimeout, ProcessChild, StderrTail, StopOutcome, WorkingDir, build_command,
};
pub use rpc::dispatch;
pub use serve::run_stdio;

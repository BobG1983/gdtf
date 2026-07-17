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
//! - [`base64`] / [`error`] — the image-content encoder and the link error vocabulary.
//!
//! # Tools
//!
//! Five tools map 1:1 onto [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest)s:
//! `send_input`, `query_state`, `get_output`, `take_screenshot`, and `app_flow` (the
//! [`mcp::tools`] registry names each request it forwards). The `start_battle` tool is
//! deliberately NOT exposed yet — its wire request exists but the game-side consumer
//! lands in T9; adding it later is one [`ToolName`](mcp::ToolName) variant plus one
//! request-builder arm.

pub mod base64;
pub mod error;
pub mod game;
pub mod mcp;
pub mod rpc;
pub mod serve;

pub use error::McpError;
pub use game::{GameClient, GameLink, GamePort};
pub use rpc::dispatch;
pub use serve::run_stdio;

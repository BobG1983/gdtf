//! The `tools/call` handler — tool name + arguments → a game request → MCP content
//! (GTW-741, extended GTW-749).
//!
//! One place owns the tool ⇄ protocol mapping: [`build_request`] turns a call's
//! arguments into a [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest), the
//! [`GameLink`](crate::game::GameLink) carries it to the game, and [`render_response`]
//! turns the [`QaResponse`](gdtf_qa_protocol::envelope::QaResponse) back into an MCP
//! content block. A link failure or a game-side
//! [`QaError`](gdtf_qa_protocol::envelope::QaError) becomes an MCP tool error
//! (`isError: true`) — never a crash, and never a fabricated screenshot.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`handle`] — [`ToolCallOutcome`] + [`handle_tool_call`], the orchestration: resolve
//!   the tool, then either drive the game lifecycle or build + carry + render.
//! - [`build`] — [`build_request`], mapping a tool's `arguments` onto a `QaRequest`.
//! - [`render`] — [`render_response`], mapping a game reply onto an MCP content block.

mod build;
mod handle;
mod render;

pub use build::build_request;
pub use handle::{ToolCallOutcome, handle_tool_call};
pub use render::render_response;

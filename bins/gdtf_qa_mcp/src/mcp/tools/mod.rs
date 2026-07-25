//! The MCP tool registry — the tools this bridge exposes (GTW-741, GTW-745, GTW-749,
//! GTW-766, GTW-787, GTW-802).
//!
//! Ten tools ([`SendInput`](ToolName::SendInput) …
//! [`FocusControl`](ToolName::FocusControl)) map 1:1 onto a
//! [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest) forwarded to a running game; two
//! more ([`LaunchGame`](ToolName::LaunchGame) / [`StopGame`](ToolName::StopGame)) are
//! host-local — they start and stop the game process itself and never reach the wire. The
//! [`ToolName`] enum is the single place the tool set is enumerated: `tools/list` walks it,
//! and [`ToolName::from_wire`] resolves a `tools/call` name.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `name` — the [`ToolName`] enum, the listing order, and the wire-name mapping.
//! - `describe` — the per-tool human description a client reads to learn the workflow.
//! - `schema` — the per-tool JSON-Schema and the `tools/list` descriptor assembly.

mod describe;
mod name;
mod schema;

#[cfg(test)]
mod test;

pub use name::ToolName;
pub use schema::tools_list_result;

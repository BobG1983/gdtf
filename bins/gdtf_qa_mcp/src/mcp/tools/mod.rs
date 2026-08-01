//! The MCP tool registry — the five tools this courier exposes (GTW-741 … GTW-943).
//!
//! [`ToolName`] is the single place the tool set is enumerated: `tools/list` walks it, and
//! [`ToolName::from_wire`] resolves a `tools/call` name. There is one tool per variant, and
//! `name`'s `ALL` slice lists them in advertised order.
//!
//! # Five tools, each taking `host`
//!
//! `launch`, `stop`, `logs`, `commands`, `run`. Every one takes the same `host` argument
//! (`"game"` or `"editor"`, defaulting to the game), so which child a call reaches is the
//! CALL's to say rather than the tool's. That is what removed the per-host duplication the
//! registry used to carry — `launch_game` beside `launch_editor`, and sixteen tools where
//! twelve were one request each.
//!
//! Nothing here names a COMMAND — not in a schema, not in a description. A host's command
//! vocabulary is read from the running host and carried as data, so adding a command to
//! either host changes nothing in this registry, needs no courier rebuild, and needs no MCP
//! reconnect. That property is what the whole command layer exists for, and
//! `test/courier.rs` pins it.
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

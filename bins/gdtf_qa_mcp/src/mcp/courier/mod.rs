//! The COURIER — the tools that carry a host's command layer to an MCP client (GTW-942,
//! widened to the whole dispatch by GTW-943).
//!
//! `commands` asks a host what it offers and renders whatever comes back; `run` carries a
//! name, an opaque JSON body and the per-call riders across and renders whatever outcome
//! comes back. Neither names a command, neither validates a command's arguments, and neither
//! has to change when a host gains one — which is the whole point of carrying a command as
//! DATA inside two frozen wire variants.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`handle`] — the `tools/call` dispatch: which host a call names, and which of the five
//!   tools it runs.
//! - [`commands`] — the catalogue read: the detail level, the optional one-command filter,
//!   and the rendering.
//! - [`run`] — the call: the argument parse into a
//!   [`RunCommand`](gdtf_qa_protocol::message::RunCommand), and the outcome rendering.
//! - [`attach`] — the files a reply hands back, resolved against the CHILD's directory and
//!   emitted as their own content blocks.

pub(super) mod attach;
pub(super) mod commands;
pub(super) mod handle;
pub(super) mod run;

pub use handle::{ToolCallOutcome, handle_tool_call};

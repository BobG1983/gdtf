//! The COURIER — the two tools that carry a host's command layer to an MCP client
//! (GTW-942).
//!
//! Everything else in [`mcp`](crate::mcp) maps one tool onto one request shape it knows the
//! meaning of. These two know almost nothing: `commands` asks a host what it offers and
//! renders whatever comes back, and `run` carries a name, an opaque JSON body and the
//! per-call riders across and renders whatever outcome comes back. Neither names a command,
//! neither validates a command's arguments, and neither has to change when a host gains one
//! — which is the whole point of carrying a command as DATA inside two frozen envelope
//! variants.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`commands`] — the catalogue read: the detail level, the optional one-command filter,
//!   and the rendering.
//! - [`run`] — the call: the argument parse into a
//!   [`RunCommand`](gdtf_qa_protocol::envelope::RunCommand), and the outcome rendering.
//! - [`attach`] — the files a reply hands back, resolved against the CHILD's directory and
//!   emitted as their own content blocks.

pub(super) mod attach;
pub(super) mod commands;
pub(super) mod run;

pub(in crate::mcp) use commands::{parse_detail, parse_filter, render_catalogue};
pub(in crate::mcp) use run::{parse_run, render_outcome};

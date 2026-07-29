//! Registry pins, one file per change-reason (GTW-905).
//!
//! - `advertised` — what `tools/list` puts in front of a client.
//! - `routing` — the wire-name mapping and the host each tool acts on.
//! - `module_doc` — the parent module doc and its counts, pinned against `ALL`.

mod advertised;
mod module_doc;
mod routing;

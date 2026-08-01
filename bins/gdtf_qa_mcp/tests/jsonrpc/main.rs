//! JSON-RPC fixture tests — literal request strings through the REAL dispatch (GTW-741;
//! split into a directory suite in GTW-808, when the four editor tools pushed the single
//! file past the 300-line warn band).
//!
//! Each test feeds a literal JSON-RPC 2.0 request line to `gdtf_qa_mcp::dispatch` and
//! asserts on the response, exercising `initialize`, `tools/list`, and `tools/call`
//! without any real game or editor (canned links).
//!
//! GTW-943 cut the per-request tool suites with the tools they drove; what is left drives
//! the five that remain.
//!
//! - `support` — the canned links, the canned lifecycles, and the dispatch helpers.
//! - `protocol` — the JSON-RPC surface itself: handshake, tool list, errors, ping.
//! - `host_local` — `launch` / `stop` / `logs`: the `host` argument that aims them, and what
//!   each outcome renders (GTW-943).
//! - `screenshot_cwd` — a capture written by a child running in ANOTHER directory relays as
//!   an image, for both children (GTW-923).
//! - `courier_tools` — the two command-layer tools: the catalogue at both detail levels and
//!   every outcome a `run` can answer with (GTW-942).
//! - `courier_riders` — the `await_ready` / `capture` riders: that each reaches the host, and
//!   carries its value (GTW-942).
//! - `courier_attach` — what a command's ATTACHMENTS render as: the image block, its order,
//!   and the child-directory read they share with `screenshot_cwd` (GTW-942).

mod courier_attach;
mod courier_riders;
mod courier_tools;
mod host_local;
mod protocol;
mod screenshot_cwd;
mod support;

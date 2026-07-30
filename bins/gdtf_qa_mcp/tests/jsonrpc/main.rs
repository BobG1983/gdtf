//! JSON-RPC fixture tests — literal request strings through the REAL dispatch (GTW-741;
//! split into a directory suite in GTW-808, when the four editor tools pushed the single
//! file past the 300-line warn band).
//!
//! Each test feeds a literal JSON-RPC 2.0 request line to `gdtf_qa_mcp::dispatch` and
//! asserts on the response, exercising `initialize`, `tools/list`, and `tools/call`
//! without any real game or editor (canned links).
//!
//! - `support` — the canned links, the canned lifecycles, and the dispatch helpers.
//! - `protocol` — the JSON-RPC surface itself: handshake, tool list, errors, ping.
//! - `game_tools` — a forwarding and a lifecycle call against the GAME.
//! - `editor_tools` — the four editor tools (GTW-808).
//! - `screenshot_host` — `take_screenshot`'s per-call `host` routing (GTW-880).
//! - `screenshot_cwd` — a capture written by a child running in ANOTHER directory relays as
//!   an image, for both children (GTW-923).

mod editor_tools;
mod game_tools;
mod protocol;
mod screenshot_cwd;
mod screenshot_host;
mod support;

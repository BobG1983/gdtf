//! [`render_response`] — a host's reply → the MCP content block for the tool that asked
//! (GTW-741; split into a directory module in GTW-808, when the editor query arms pushed
//! the single file past the 300-line warn band).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `response` — the reply → content-block mapping itself.
//! - `shot_file` — where a saved capture is read from: the child's reported path resolved
//!   against the directory the CHILD ran in, never the MCP host's own (GTW-923).

mod response;
mod shot_file;

#[cfg(test)]
mod test;

pub use response::render_response;

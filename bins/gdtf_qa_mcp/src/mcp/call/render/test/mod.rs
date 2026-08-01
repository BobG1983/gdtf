//! The reply → content-block mapping's unit tests (GTW-741 onward; split into a directory
//! module in GTW-923, when the capture-path tests pushed the single file past the 300-line
//! warn band).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `replies` — the non-capture arms: errors, receipts, views, and the editor queries.
//! - `screenshot` — the two capture arms: which file is read, and every refusal that must
//!   stay a tool error rather than an image.
//!
//! - `args` — the empty call-arguments fixture both files render with.
//!
//! The COURIER's two arms are covered in `tests/jsonrpc/`, through the real dispatch, since
//! what matters about them is the JSON a client actually receives.

mod args;
mod replies;
mod screenshot;

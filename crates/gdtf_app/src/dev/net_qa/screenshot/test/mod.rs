//! Tests for the T7 screenshot service (GTW-740).
//!
//! - [`path`] — pure confinement + uniqueness tests over the REAL default
//!   `target/qa_screenshots` directory (traversal / absolute / nested stripping, default
//!   fallback, the sequenced per-capture path).
//! - [`pump`] — headless pump tests on the REAL
//!   [`drive_screenshots`](super::pump::drive_screenshots) pump: the deferred reply (never
//!   on the claim frame), a PNG landing AFTER the claim frame reported as a landed capture,
//!   a STALE PNG at the path purged + never served, and an existing-but-UNDECODABLE file
//!   rejected.

mod path;
mod pump;

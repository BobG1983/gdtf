//! Guard: no tracked Rust file hands the frame clock back to Bevy, so a test that pins a
//! manual frame delta leaves it pinned.

mod check;
mod message;
mod scan;
mod tree;

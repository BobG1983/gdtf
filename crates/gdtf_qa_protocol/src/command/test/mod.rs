//! Exhaustive round-trip suites for the command vocabulary (GTW-939, GTW-942).
//!
//! - `round_trip` — every GTW-939 value survives `ron::ser` → `ron::de` unchanged.
//! - `timing` — the GTW-942 [`CommandTiming`](crate::command::CommandTiming) field and the
//!   backward-compatible decode its `#[serde(default)]` promises.

mod round_trip;
mod timing;

//! Unit tests for the pending map and its deadline pump (GTW-803).
//!
//! - [`sweep`] — the unclaimed-entry timeout and the claimed-entry pass-through, driven
//!   through the REAL [`sweep_pending`](crate::sweep_pending) system in a real `App`.

mod sweep;

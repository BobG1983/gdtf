//! Game-lifecycle integration — the REAL launch / stop logic (GTW-745), and WHEN a failed
//! launch reads the child's stderr tail (GTW-756).
//!
//! The [`GameManager`](gdtf_qa_mcp::GameManager), its readiness poll, timeout handling,
//! and SIGTERM→SIGKILL→reap stop sequence are the actual production code throughout; only
//! the externals are supplied by the test, from the shared harness in [`support`]. Two
//! concerns, one file each:
//!
//! - [`process`] drives the manager against a REAL placeholder child process and a fake
//!   `net_qa` listener: a launch that becomes ready and stops, the ensure-style second
//!   launch, the boot timeout that kills the orphan and carries its captured stderr tail,
//!   and the no-op stop.
//! - [`tail_order`] pins the ORDER the two failure paths read that tail in — after the
//!   child is reaped, never before — against a processless fake child that makes the
//!   ordering observable with no clock in the assertion.

mod process;
mod support;
mod tail_order;

//! Game-lifecycle integration — the REAL launch / stop logic (GTW-745), and WHEN a failed
//! launch reads the child's stderr tail (GTW-756).
//!
//! The [`HostManager`](gdtf_qa_mcp::HostManager), its readiness poll, timeout handling,
//! and SIGTERM→SIGKILL→reap stop sequence are the actual production code throughout; only
//! the externals are supplied by the test, from the shared harness in [`support`]. Three
//! concerns, one file each:
//!
//! - [`process`] drives the manager against a REAL placeholder child process and a fake
//!   `net_qa` listener: a launch that becomes ready and stops, the ensure-style second
//!   launch, the boot timeout that kills the orphan and carries its captured stderr tail,
//!   and the no-op stop.
//! - [`recipe`] drives the REAL [`CargoSpawner`](gdtf_qa_mcp::CargoSpawner) to prove a
//!   launch recipe's working directory is where cargo actually runs, and that successive
//!   launches can name different recipes (GTW-875).
//! - [`child_dir`] asks the real manager WHERE its running child is, the fact the render
//!   path reads to open a capture the child wrote at a relative path (GTW-923).
//! - [`tail_order`] pins the ORDER the two failure paths read that tail in — after the
//!   child is reaped, never before — against a processless fake child that makes the
//!   ordering observable with no clock in the assertion.

mod child_dir;
mod process;
mod recipe;
mod support;
mod tail_order;

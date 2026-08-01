//! Game-lifecycle integration — the REAL launch / stop logic (GTW-745), and WHEN a failed
//! launch reads the child's captured output tail (GTW-756).
//!
//! The [`HostManager`](gdtf_qa_mcp::HostManager), its readiness poll, timeout handling,
//! and SIGTERM→SIGKILL→reap stop sequence are the actual production code throughout; only
//! the externals are supplied by the test, from the shared harness in [`support`]. Three
//! concerns, one file each:
//!
//! - [`process`] drives the manager against a REAL placeholder child process and a fake
//!   `net_qa` listener: a launch that becomes ready and stops, the ensure-style second
//!   launch, the boot timeout that kills the orphan and carries its captured output tail,
//!   and the no-op stop.
//! - [`recipe`] drives the REAL [`CargoSpawner`](gdtf_qa_mcp::CargoSpawner) to prove a
//!   launch recipe's working directory is where cargo actually runs, and that successive
//!   launches can name different recipes (GTW-875).
//! - [`child_dir`] asks the real manager WHERE its running child is, the fact the render
//!   path reads to open a capture the child wrote at a relative path (GTW-923).
//! - [`orphan`] drives the manager over a port a live listener genuinely holds while it
//!   owns no child — the state an MCP host restart leaves behind — so a stop reports the
//!   orphan instead of `not_running` and a launch reports it instead of racing it
//!   (GTW-926).
//! - [`production_wiring`] pins the one line those orphan tests cannot reach — the watch
//!   [`HostManager::with_config`](gdtf_qa_mcp::HostManager::with_config) supplies, which is
//!   how the shipped MCP host builds both of its managers.
//! - [`system_watch`] covers the other half of that state — the REAL
//!   [`SystemOrphanWatch`](gdtf_qa_mcp::SystemOrphanWatch): what its lookup names, and what
//!   its stop actually signals (the named process, whatever group it leads, AND anything
//!   that process spawned into its group), against placeholder processes the test spawns
//!   itself. It is Unix-only because the stop shells out to `kill(1)`, which only exists
//!   there.
//! - [`output_tail`] drives a REAL child that writes to BOTH of its streams and reads the
//!   capture back: stdout is no longer discarded, and a line cap returns the newest lines
//!   (GTW-943 — the plumbing the `logs` tool stands on).
//! - [`child_output`] asks the real manager what its running child printed — the one line
//!   that joins the `logs` tool to that ring, and the only place a dropped line cap or a
//!   `None` for a live child is visible (GTW-943).
//! - [`tail_order`] pins the ORDER the two failure paths read that tail in — after the
//!   child is reaped, never before — against a processless fake child that makes the
//!   ordering observable with no clock in the assertion.

mod child_dir;
mod child_output;
mod orphan;
mod output_tail;
mod process;
mod production_wiring;
mod recipe;
mod support;
#[cfg(unix)]
mod system_watch;
mod tail_order;

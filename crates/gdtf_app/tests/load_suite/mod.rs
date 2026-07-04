//! The GTW-580 shared load-suite module — the generic per-family Load-test
//! harness every folder-family wrapper (`tests/load_<family>.rs`) includes via
//! `mod load_suite;`.
//!
//! Submodules by concern (wiring only here):
//!
//! - [`gate`] — the shared Load-gate seed helpers over the ONE seed source
//!   (`gdtf_app::test_support::seed_load_gate`). Bespoke, non-family load tests
//!   include it standalone via `#[path = "load_suite/gate.rs"] mod gate;`.
//! - [`suite`] — the parameterized tier-(a)/(b) suite plus the
//!   [`FamilyLoadContract`](suite::FamilyLoadContract) trait each wrapper
//!   implements for its `gdtf_content_families` marker. The full "family N+1"
//!   recipe lives on that module's rustdoc.

pub(crate) mod gate;
pub(crate) mod suite;

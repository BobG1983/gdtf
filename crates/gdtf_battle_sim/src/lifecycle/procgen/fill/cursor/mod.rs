//! The **fill sub-cursor** (GTW-732) — the resumable [`FillCursor`](driver::FillCursor) whose
//! one `step` places exactly ONE fill prefab, lifting all the state that was local to
//! `fill_placement_with`'s call frame into a value that survives across single-placement steps.
//!
//! `state` holds the state-machine vocabulary (the per-step yield, the sub-pass enum, the
//! scatter slot bookkeeping); `driver` holds the cursor value + its stepping algorithm.
//! `mod.rs` is wiring-only.

mod driver;
mod state;

pub(in crate::lifecycle::procgen) use driver::FillCursor;
pub(in crate::lifecycle::procgen) use state::FillStep;

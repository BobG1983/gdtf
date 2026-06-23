//! The squad-visible TARGETING GATE (GTW-11): the ONE shared squad-visible predicate
//! the reticle, the hint, and the fire-refusal all consume so they can never disagree
//! (`docs/combat/visibility.md` §"UX edges").
//!
//! `docs/combat/resolution.md` §"What's pure math vs sim" pins the fog gate as
//! **player policy** — "unseen — hold your fire" is presenter / input policy and
//! NEVER enters the shared `can_fire` act (which stays LOS / fog-free). This module
//! is the presenter-owned home of that policy: it DEFINES the [`CellVisibility`]
//! verdict (a NAMED domain value, no-bare-types) carried in the [`HighlightRequest`](crate::HighlightRequest)
//! and the pure [`cell_squad_visible`] predicate that decides it. The presenter
//! DEFINES the type (the consumer owns its input API, the [`HighlightRequest`](crate::HighlightRequest)
//! precedent); the input crate consumes it for the reticle + the fire-refusal rung,
//! and the app consumes it for the targeting hint.

mod gate;

pub use gate::{CellVisibility, cell_squad_visible};

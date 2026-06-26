//! The automatic positional [`PeekOffset`](crate::los::PeekOffset) populator — GTW-406,
//! the producer half of the GTW-393 wall-peek seam.
//!
//! GTW-393 ([`crate::los`]) shipped the [`PeekOffset`](crate::los::PeekOffset) component
//! and the [`has_los_peeking`](crate::los::has_los_peeking) consumer but **no populator**
//! — the offset was non-zero only in direct-set tests. This module is that populator: a
//! change-driven sim-internal maintenance layer (the `occupancy_sync` thesis — react to
//! the *change*, edit in place) that AUTOMATICALLY derives each ganger's peek from where
//! it stands.
//!
//! Two pieces:
//!
//! 1. `corner_lean` (the private `corner` module) — the pure, target-agnostic corner
//!    geometry: a [`CellLevel`](crate::metric::CellLevel) + an
//!    [`OccupancyGrid`](crate::occupancy::OccupancyGrid) → the single-axis lean toward a
//!    corner's open edge (or zero). Render-free, ECS-free, unit-testable.
//! 2. [`sync_peek_offsets`] + [`peek_population_needed`] (the private `systems` module) —
//!    the Bevy system that writes `corner_lean` into every ganger's
//!    [`PeekOffset`](crate::los::PeekOffset) IN PLACE, and its run-condition gating it on
//!    a move/spawn (`Changed<Position>`) or a wall/cover destruction
//!    ([`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed)).
//!
//! It writes the **existing** [`PeekOffset`](crate::los::PeekOffset) component, so the
//! `Changed<PeekOffset>` recompute trigger (recompute.rs:41) and
//! [`has_los_peeking`](crate::los::has_los_peeking) pick the populated value up for free.
//!
//! **No live consumer reads the populated peek yet** (GTW-406 §E, honestly surfaced): the
//! squad fog ([`union_fov`](crate::visibility::union_fov)) is centred-eye by GTW-393 canon
//! and [`has_los_peeking`](crate::los::has_los_peeking) has no live caller. GTW-406 ships
//! the producer + an end-to-end test; the live consumer is the AI / reaction-fire /
//! fire-targeting epic (a filed follow-up).
//!
//! **Wiring lives in [`BattleSimPlugin`](crate::battle::BattleSimPlugin)**, not a plugin
//! here: [`sync_peek_offsets`] is added to the [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems)
//! band ordered `.after(advance_walk).after(sync_destroyed_cover).before(recompute_visibility)`
//! and gated on [`peek_population_needed`] — so the populated offset is consumed by the
//! same tick's recompute.

mod corner;
mod systems;

#[cfg(test)]
mod test;

pub use systems::{peek_population_needed, sync_peek_offsets};

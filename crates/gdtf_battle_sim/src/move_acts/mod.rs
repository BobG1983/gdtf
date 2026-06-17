//! The **movement verb** — the FIRST movement act in the authoritative sim (GTW-234).
//!
//! [`move_ganger`] is the pure, render-free verb that steps a ganger one cell to a
//! destination `(cell, level)`, charging a **terrain-determined** TU cost. It mirrors the
//! [`crate::posture`] / [`crate::downed_acts`] verb-module precedent: it takes the landed
//! ganger newtypes by reference plus the read-only
//! [`OccupancyGrid`](crate::occupancy::OccupancyGrid) and the
//! [`MoveCosts`](crate::tuning::MoveCosts) tuning table, with **no
//! [`World`](bevy::ecs::world::World) access**, so it unit-tests against bare component
//! values with no ECS plumbing. The message-driven seam ([`crate::acts::MoveRequested`] +
//! `dispatch_move`) lives in [`crate::acts`]; this module owns the rule.
//!
//! ## Terrain-determined cost (the core)
//!
//! The move's TU cost is NOT a flat per-cell constant — it is the DESTINATION cell's
//! terrain movement cost (`docs/combat/combat.md` L34: "step" costs TUs; the user ruling
//! "the floor tile you cross — the terrain determines the cost"). The verb reads
//! [`OccupancyGrid::terrain`](crate::occupancy::OccupancyGrid::terrain) at the
//! destination, looks its cost up in the per-[`TerrainKind`](crate::occupancy::TerrainKind)
//! [`MoveCosts`](crate::tuning::MoveCosts) table, and charges exactly that. The
//! granularity is per-`TerrainKind` (coarse — Open / Cover / Wall); richer
//! per-floor-type costs are a follow-up. The cost is applied flat regardless of step
//! direction (a diagonal-costs-more differential is deferred).
//!
//! ## The gates (ALL must pass, or it is a TOTAL no-op)
//!
//! A move succeeds only when EVERY gate holds — and the pass/fail decision is computed
//! BEFORE any mutation, so a failing move touches NEITHER
//! [`Position`](crate::ganger::Position) NOR [`Tu`](crate::ganger::Tu):
//!
//! - the actor is [`LifeState::Alive`](crate::ganger::LifeState::Alive) (a Downed / Dead
//!   ganger cannot move);
//! - the destination is **in-bounds** (a real in-grid `(cell, level)`,
//!   [`OccupancyGrid::slot`](crate::occupancy::OccupancyGrid::slot) returns `Some`) AND
//!   **not blocked** ([`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked)
//!   is `false` — a standing Wall / Cover cell is rejected; a destroyed-cover cell passes,
//!   its terrain still reads [`TerrainKind::Cover`](crate::occupancy::TerrainKind::Cover) so
//!   its per-terrain cost still applies);
//! - the destination is **unoccupied**
//!   ([`OccupancyGrid::occupant`](crate::occupancy::OccupancyGrid::occupant) is `None`);
//! - the move is **affordable** ([`can_spend_tu`](crate::tu::can_spend_tu) against the
//!   looked-up terrain cost).
//!
//! Occupancy beats affordability: an affordable move onto an occupied cell is a no-op.
//! On success the verb writes `Position = Position::new(dest)` and spends the looked-up
//! cost via [`spend_tu`](crate::tu::spend_tu) — and writes ONLY
//! [`Position`](crate::ganger::Position): the occupancy-grid slot maintenance is the
//! landed [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) reactor on
//! `Changed<`[`Position`](crate::ganger::Position)`>`, never a grid write here.

#[cfg(test)]
mod test;
mod verb;

pub use verb::{MoveOutcome, move_ganger};

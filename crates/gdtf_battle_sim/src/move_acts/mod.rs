//! The **movement verb** — the movement act in the authoritative sim (E7 · GTW-355,
//! evolving the original GTW-234 single-step `move_ganger` verb).
//!
//! [`advance_walk`] is the render-free walk system that drives an accepted route ONE cell
//! per tick — the committed step-by-step walk that replaced the original single-jump verb
//! (the old any-cell `move_ganger` step that charged a **terrain-determined** TU cost up
//! front). The message-driven seam ([`crate::acts::MoveRequested`] + `dispatch_move`)
//! lives in [`crate::acts`]: `dispatch_move` plans + gates a reachable, affordable route
//! and attaches a [`WalkInProgress`]; this module owns the per-tick stepping rule.
//!
//! ## Terrain-determined cost (the core)
//!
//! The walk's TU cost is NOT a flat per-cell constant — each entered cell costs its own
//! terrain movement cost (`docs/combat/combat.md` L34: "step" costs TUs; the user ruling
//! "the floor tile you cross — the terrain determines the cost"). The route's per-step
//! entry costs are looked up from the destination cells' terrain in the per-`TerrainKind`
//! [`MoveCosts`](crate::tuning::MoveCosts) table at plan time and charged atomically with
//! each [`Position`](crate::ganger::Position) write as the walk advances. The granularity
//! is per-`TerrainKind` (coarse — Open / Cover / Wall); richer per-floor-type costs are a
//! follow-up.
//!
//! ## The gates
//!
//! A route is planned and gated affordable up front by `dispatch_move` (liveness +
//! reachability + full-route affordability); each subsequent step then bump-stops on live
//! obstacles and halts the moment an enemy is revealed or a reaction shot interrupts.
//! Every step writes ONLY [`Position`](crate::ganger::Position): the occupancy-grid slot
//! maintenance is the landed
//! [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) reactor on
//! `Changed<`[`Position`](crate::ganger::Position)`>`, never a grid write here.

mod walk;

pub use walk::{ReactionShotFired, WalkInProgress, advance_walk};

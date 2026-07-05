//! The **move** dispatch — the SINGLE writer that, on each buffered [`MoveRequested`](crate::acts::request::MoveRequested)
//! commit, plans a reachable affordable route and (only then) starts the committed walk
//! (E7 · GTW-12f / GTW-354 / GTW-355; the original any-cell dispatch was E4 / GTW-234).
//!
//! ## What this slice adds (GTW-354)
//!
//! Before GTW-354 the dispatch let the original single-step `move_ganger` verb jump to ANY
//! single empty in-bounds cell — an any-empty-cell teleport (the destination need not be
//! adjacent or reachable). GTW-354 makes [`dispatch_move`] the single CONSTRAINED writer:
//! on each [`MoveRequested`](crate::acts::request::MoveRequested) (the COMMIT — see below) it runs [`find_path`](crate::pathfinder::find_path) from the
//! mover's cell to the requested [`CellLevel`](crate::metric::CellLevel) through the GTW-353 visibility-gated
//! [`PlanningView`](crate::pathfinder::PlanningView) and:
//!
//! - **REJECTS** the move (a TYPED [`MoveRejected`], NO step) when no route exists
//!   ([`PathBlocked`](crate::pathfinder::PathBlocked) — the teleport is dead); and
//! - performs ONE up-front **full-route affordability** gate (`docs/combat/visibility.md`
//!   §48 — "the commit gates full-route affordability once, up front") against the
//!   [`Path::total`](crate::pathfinder::Path::total), rejecting (TYPED [`MoveRejected`], NO step) when the mover cannot
//!   afford the whole route.
//!
//! Only when a route exists AND is affordable does it accept: per GTW-355 it attaches a
//! [`WalkInProgress`](crate::move_acts::WalkInProgress) holding the planned route ahead (each cell's DESTINATION-terrain
//! per-step charge held verbatim), which [`advance_walk`](crate::move_acts::advance_walk)
//! then walks ONE cell per tick, plus the [`MovementOccurred`] log signal. No act logic is
//! reimplemented and the per-step charge is UNTOUCHED; the up-front gate here is a CHECK
//! against the planned total, NOT a second charge.
//!
//! ## Commit semantics (C2 — cross-ticket boundary)
//!
//! [`dispatch_move`] dispatches on [`MoveRequested`](crate::acts::request::MoveRequested), which IS the commit (the 2nd /
//! commit click). It does NOT dispatch on a select / preview. The two-click INPUT (click-1
//! select+preview vs click-2 commit) is **GTW-356** and the preview DISPLAY is **GTW-358**
//! — NOT this slice. This dispatch treats every drained [`MoveRequested`](crate::acts::request::MoveRequested) as a
//! commit-dispatch; it does no click-counting (that is GTW-356's input concern).
//!
//! It fetches the actor's components + reads the grids via Bevy queries / `Res`
//! (`bevy-traps.md` #7 — no `&mut World`); it only READS the grids + the squad fog.
//!
//! ## GTW-444 — "Hampered" movement-cost factor
//!
//! The dispatch reads the mover's
//! [`MovementCostFactor`](crate::injuries::MovementCostFactor) from its
//! [`InflictedInjuries`](crate::injuries::InflictedInjuries) ledger and passes it to
//! [`find_path`](crate::pathfinder::find_path), which scales EVERY planar per-step floor cost by it. Because the
//! accepted [`WalkInProgress`](crate::move_acts::WalkInProgress) holds the planned [`Path::steps`](crate::pathfinder::Path::steps)
//! VERBATIM and [`advance_walk`](crate::move_acts::advance_walk) charges those steps, the
//! factor flows through to the actual per-step TU charge with NO second application — so a
//! Hampered unit's previewed path cost equals the TU it is charged (preview==charge, C3).
//! An uninjured mover passes the IDENTITY factor (`1.0`), leaving the cost unchanged.

mod dispatch;
mod signals;
mod suppression_gate;

pub use dispatch::dispatch_move;
pub use signals::{MoveRejected, MoveRejection, MovementOccurred};

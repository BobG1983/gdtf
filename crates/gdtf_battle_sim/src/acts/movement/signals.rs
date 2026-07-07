//! The move act's typed output signals — the [`MoveRejection`] reasons, the
//! [`MoveRejected`] no-step signal, and the per-step [`MovementOccurred`] log signal.

use bevy::prelude::{Entity, Message};

use crate::metric::Cell;

/// Why a [`MoveRequested`](crate::acts::request::MoveRequested) commit was **rejected** — the two no-step outcomes of the
/// GTW-354 route + affordability gate (C3).
///
/// A named domain enum (no-bare-types: a move's rejection reason is a domain value, not a
/// bare flag), mirroring the [`ReloadOutcome`](crate::acts::ReloadOutcome) shape. The
/// presenter / any reactive system classifies a [`MoveRejected`] by this variant. A move
/// that SUCCEEDS emits no [`MoveRejected`] (it emits [`MovementOccurred`] instead), so
/// there is no "accepted" variant here — and an actor missing a queried component is an
/// internal guard SKIP (no rejection signal at all, the `dispatch_*` precedent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveRejection {
    /// No route exists from the mover's cell to the requested destination over the
    /// visibility-routable grid + links ([`find_path`](crate::pathfinder::find_path) returned
    /// [`PathBlocked`](crate::pathfinder::PathBlocked)) — the destination is unreachable
    /// (it may be off-grid, walled off, behind an UNSEEN region, or blocked by a visible
    /// ganger). This is what kills the pre-GTW-354 any-empty-cell teleport.
    Unreachable,
    /// A route exists but the mover cannot afford its full-route [`Tu`](crate::ganger::Tu) cost — the single
    /// up-front affordability gate (`docs/combat/visibility.md` §48) failed against the
    /// planned [`Path::total`](crate::pathfinder::Path::total). NO partial move: the mover
    /// stays put and spends nothing (GTW-355 owns the stepped walk; this is a CHECK, not a
    /// charge).
    Unaffordable,
    /// The mover is [`Suppressed`](crate::ganger::Suppressed) and the chosen destination is ILLEGAL for a pinned unit
    /// (GTW-537, child GTW-41a of GTW-41; `docs/combat/combat.md` "Suppression … advanced
    /// combat effects"). A suppressed mover may ONLY step to a destination that is BOTH (a)
    /// STRICTLY FARTHER from the [`SuppressorCell`](crate::ganger::SuppressorCell) than its
    /// start cell (measured with the sim's Chebyshev ground-plane metric), AND (b) BEHIND
    /// COVER relative to the suppressor (the cell one step from the destination TOWARD the
    /// suppressor holds registered cover in the [`CoverLedger`](crate::cover::CoverLedger)). A destination failing
    /// EITHER clause is a HARD REJECT (no clamp) — NO step. On a cover-sparse map this can
    /// pin the unit hard; that is the intended "pinned" feel (GTW-537 R1). An UNSUPPRESSED
    /// mover is never subject to this gate (identity).
    Suppressed,
}

/// A **move was rejected** — the typed no-step signal that `actor`'s commit could not be
/// dispatched, with the [`MoveRejection`] reason (GTW-354, C3).
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`ReloadResult`](crate::acts::ReloadResult) / [`MovementOccurred`]. Emitted ONCE per
/// drained [`MoveRequested`](crate::acts::request::MoveRequested) whose route gate fails — either no route
/// ([`MoveRejection::Unreachable`]) or an unaffordable route
/// ([`MoveRejection::Unaffordable`]). Neither [`Position`](crate::ganger::Position) nor [`Tu`](crate::ganger::Tu) is touched on a
/// reject (a TOTAL no-op). The [`actor`](MoveRejected::actor) is a Bevy [`Entity`] handle
/// — framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRejected {
    /// The ganger whose move commit was rejected.
    pub actor:  Entity,
    /// Why the commit was rejected (no route, or an unaffordable route).
    pub reason: MoveRejection,
}

impl MoveRejected {
    /// Build a move-rejected signal for `actor` with the given [`MoveRejection`] reason.
    #[must_use]
    pub const fn new(actor: Entity, reason: MoveRejection) -> Self {
        Self { actor, reason }
    }
}

/// A **move occurred** — the combat-log signal that `actor` stepped from `from` to `to`
/// (GTW-328), emitted ONCE per accepted WALK STEP (GTW-355).
///
/// The combat-text LOG event for a move ("`<name>` moved `<from>` -> `<to>`") — the user-facing
/// announcement that a ganger changed cell. Since GTW-355 the committed walk
/// ([`advance_walk`](crate::acts::movement::advance_walk)) emits ONE of these per DISCRETE step
/// it takes, so a multi-cell walk announces a step per cell entered; a rejected
/// (unreachable / unaffordable) commit emits a [`MoveRejected`] and no `MovementOccurred`,
/// and a bump-stopped / interrupted walk simply stops emitting them. The
/// [`from`](MovementOccurred::from) cell is the actor's pre-step ground cell and
/// [`to`](MovementOccurred::to) the cell it entered, so they are the actual pre/post ground
/// cells of THAT step. It adds **no** act logic and re-resolves nothing — pure exposure of
/// the step the walk already performed.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`crate::acts::ReloadResult`]. The [`actor`](MovementOccurred::actor) is a Bevy
/// [`Entity`] handle — framework plumbing, the only bare type the no-bare-types rule
/// permits in a payload; [`from`](MovementOccurred::from) / [`to`](MovementOccurred::to)
/// are the domain [`Cell`] newtype.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementOccurred {
    /// The ganger that stepped — resolved to a name by the combat-log presenter via
    /// `Query<&GangerName>`.
    pub actor: Entity,
    /// The ground [`Cell`] the actor stepped FROM (its pre-step cell).
    pub from:  Cell,
    /// The ground [`Cell`] the actor stepped TO (the cell it entered this step).
    pub to:    Cell,
}

impl MovementOccurred {
    /// Build a movement-occurred signal for `actor` stepping from `from` to `to`.
    #[must_use]
    pub const fn new(actor: Entity, from: Cell, to: Cell) -> Self {
        Self { actor, from, to }
    }
}

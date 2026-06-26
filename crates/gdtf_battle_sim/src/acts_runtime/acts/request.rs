//! The `*Requested` buffered [`Message`] types — the message-driven **input
//! contract** for the landed combat acts (E10.2 / GTW-204; the seventh,
//! [`MoveRequested`], added in GTW-234; the eighth, [`ReloadRequested`], in GTW-275; the
//! ninth, the fieldless [`EndTurnRequested`] turn signal, in GTW-309).
//!
//! Eight [`#[derive(Message)]`](bevy::prelude::Message) buffered messages — mirroring
//! [`crate::bleed::Bleeding`] / [`crate::occupancy_sync::CoverDestroyed`], the buffered
//! `Message` API, NOT the observer `Event` API (`bevy-traps.md` #4). Each carries the
//! act's [`Entity`] actor ref(s) plus the act's OWNED payload. A `Message` cannot hold a
//! borrow, so [`FireRequested`] carries an OWNED [`FireModeSpec`] (now `Copy` again,
//! GTW-260) plus the target [`Cell`] / [`Level`] — the type has **no lifetime
//! parameter**; the [`dispatch_fire`](super::fire::dispatch_fire) system reconstructs the
//! borrow-based [`FireOrder`](crate::fire::FireOrder) `{ mode: &owned_spec, target_cell,
//! target_level }` from the owned payload at the call site.
//!
//! These carry [`Entity`] actor refs (matching the landed `fire(shooter: Entity)` and
//! the downed verbs' actor/target entities) — NOT presenter-facing integer ids. A
//! `*Resolved` integer-id boundary is a LATER epic; E10 relies on component
//! change-detection for the view, so this slice ships no `*Resolved` types.

use bevy::prelude::{Deref, Entity, Message};

use crate::{
    ganger::{Direction, StanceKind},
    metric::{Cell, CellLevel, Level},
    weapon::FireModeSpec,
};

/// The requested **aim flag** carried by a [`SetAimingRequested`] — `true` for aimed
/// fire, `false` for hip-fired.
///
/// A no-bare-types newtype over the aim-mode `bool` payload (a domain value — the
/// requested aim mode, not framework plumbing): a private inner + a derived [`Deref`],
/// the crate's newtype house style. Distinct from the
/// [`Aiming`](crate::ganger::Aiming) *component* (the ganger's current aim state): this
/// is the *requested* value the dispatch verb sets the component to.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimRequest(bool);

impl AimRequest {
    /// Build a requested aim flag — `true` to aim, `false` to hip-fire.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// A **fire** act was requested — fire `mode` at `(target_cell, target_level)` for
/// `shooter`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] shooter ref plus the act's OWNED payload: a [`FireModeSpec`] (owned by value
/// — a `Message` cannot hold a borrow; `Copy` again, GTW-260) plus the target
/// [`Cell`] / [`Level`]. The type has **no
/// lifetime parameter**; [`dispatch_fire`](super::fire::dispatch_fire) reconstructs the
/// borrow-based [`FireOrder`](crate::fire::FireOrder) `{ mode: &mode, target_cell,
/// target_level }` from this owned payload. The shooter is a Bevy [`Entity`] handle —
/// framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct FireRequested {
    /// The firing entity (armed shooter).
    pub shooter:      Entity,
    /// The selected fire mode's per-mode numbers — OWNED (no borrow), so the message has
    /// no lifetime; the dispatch system borrows it into a [`FireOrder`](crate::fire::FireOrder).
    pub mode:         FireModeSpec,
    /// The target cell the player aimed at (the §2 aim cell's x/y).
    pub target_cell:  Cell,
    /// The target storey the player aimed at (the aim cell's z).
    pub target_level: Level,
}

impl FireRequested {
    /// Build a fire request for `shooter` firing `mode` at `(target_cell, target_level)`.
    #[must_use]
    pub const fn new(
        shooter: Entity,
        mode: FireModeSpec,
        target_cell: Cell,
        target_level: Level,
    ) -> Self {
        Self {
            shooter,
            mode,
            target_cell,
            target_level,
        }
    }
}

/// A **set-aiming** act was requested — set `actor`'s aim flag to `aim`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`AimRequest`] flag. [`dispatch_set_aiming`](super::posture::dispatch_set_aiming)
/// calls [`set_aiming`](crate::posture::set_aiming) (which spends NO TU — toggling aim is
/// free, `posture.rs`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetAimingRequested {
    /// The acting ganger whose [`Aiming`](crate::ganger::Aiming) flag is set.
    pub actor: Entity,
    /// The requested aim value the flag is set to.
    pub aim:   AimRequest,
}

impl SetAimingRequested {
    /// Build a set-aiming request for `actor` to the requested aim flag.
    #[must_use]
    pub const fn new(actor: Entity, aim: AimRequest) -> Self {
        Self { actor, aim }
    }
}

/// A **set-stance** act was requested — change `actor`'s stance to `stance`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`StanceKind`]. [`dispatch_set_stance`](super::posture::dispatch_set_stance) calls
/// [`set_stance`](crate::posture::set_stance), which charges the
/// [`StanceChangeTu`](crate::tuning::StanceChangeTu) tuning leaf ONLY on a real change
/// (a no-op, no charge, when the actor already holds `stance`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetStanceRequested {
    /// The acting ganger whose [`Stance`](crate::ganger::Stance) is changed.
    pub actor:  Entity,
    /// The requested posture to change to.
    pub stance: StanceKind,
}

impl SetStanceRequested {
    /// Build a set-stance request for `actor` to change to `stance`.
    #[must_use]
    pub const fn new(actor: Entity, stance: StanceKind) -> Self {
        Self { actor, stance }
    }
}

/// A **set-facing** act was requested — turn `actor` to face `facing`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`Direction`]. [`dispatch_set_facing`](super::posture::dispatch_set_facing) calls
/// [`set_facing`](crate::posture::set_facing), which charges the
/// [`TurnTu`](crate::tuning::TurnTu) tuning leaf ONLY on a real turn (a no-op, no charge,
/// when the actor already faces `facing`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetFacingRequested {
    /// The acting ganger whose [`Facing`](crate::ganger::Facing) is turned.
    pub actor:  Entity,
    /// The requested direction to turn to.
    pub facing: Direction,
}

impl SetFacingRequested {
    /// Build a set-facing request for `actor` to turn to `facing`.
    #[must_use]
    pub const fn new(actor: Entity, facing: Direction) -> Self {
        Self { actor, facing }
    }
}

/// A **stabilize-downed** act was requested — `actor` stabilizes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_stabilize_downed`](super::downed::dispatch_stabilize_downed) assembles the
/// [`Actor`](crate::downed_acts::Actor) / [`DownedTarget`](crate::downed_acts::DownedTarget)
/// bundles from the queried components and calls
/// [`stabilize_downed`](crate::downed_acts::stabilize_downed), whose faction gate
/// ([`can_stabilize`](crate::downed_acts::can_stabilize)) holds end-to-end — only an
/// 8-adjacent alive ALLY sets the target's [`Stabilized`](crate::ganger::Stabilized) flag
/// (the target stays Downed).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilizeDownedRequested {
    /// The acting (would-be stabilizer) ganger.
    pub actor:  Entity,
    /// The downed target to stabilize.
    pub target: Entity,
}

impl StabilizeDownedRequested {
    /// Build a stabilize-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// An **execute-downed** act was requested — `actor` executes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_execute_downed`](super::downed::dispatch_execute_downed) assembles the
/// [`Actor`](crate::downed_acts::Actor) / [`DownedTarget`](crate::downed_acts::DownedTarget)
/// bundles from the queried components and calls
/// [`execute_downed`](crate::downed_acts::execute_downed), whose faction gate
/// ([`can_execute`](crate::downed_acts::can_execute)) holds end-to-end — only an
/// 8-adjacent alive ENEMY transitions the target to
/// [`LifeState::Dead`](crate::ganger::LifeState::Dead).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecuteDownedRequested {
    /// The acting (would-be executor) ganger.
    pub actor:  Entity,
    /// The downed target to execute.
    pub target: Entity,
}

impl ExecuteDownedRequested {
    /// Build an execute-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// A **move** act was requested — step `actor` one cell to `dest`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] actor ref plus the destination [`CellLevel`] (the `(cell, level)` to step
/// to). The payload is OWNED and `Copy` ([`CellLevel`] is `Copy`), so the type has **no
/// lifetime parameter** — mirroring [`SetFacingRequested`] / [`SetStanceRequested`]. The
/// actor is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a payload; `dest` is the landed [`CellLevel`] newtype.
/// [`dispatch_move`](super::movement::dispatch_move) drains this and, per message, plans a
/// reachable affordable route and attaches a
/// [`WalkInProgress`](crate::move_acts::WalkInProgress) the landed
/// [`advance_walk`](crate::move_acts::advance_walk) walk drives, each step's TU cost being
/// the entered cell's terrain movement cost.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRequested {
    /// The acting ganger to step.
    pub actor: Entity,
    /// The destination `(cell, level)` to step the actor to (one cell, no pathfinding).
    pub dest:  CellLevel,
}

impl MoveRequested {
    /// Build a move request for `actor` to step to `dest`.
    #[must_use]
    pub const fn new(actor: Entity, dest: CellLevel) -> Self {
        Self { actor, dest }
    }
}

/// A **reload** act was requested — refill `actor`'s magazine, charging the weapon's
/// per-weapon reload TU cost (GTW-275).
///
/// A buffered [`Message`] carrying ONLY the [`Entity`] actor ref — the cost (the
/// [`ReloadTu`](crate::magazine::ReloadTu)) and the target fill (the
/// [`MagazineSize`](crate::weapon::MagazineSize)) both live on the actor's own
/// [`Magazine`](crate::magazine::Magazine) grouping component, so the message needs no
/// payload (the [`SetStanceRequested`] shape, minus the requested value).
/// [`dispatch_reload`](super::reload::dispatch_reload) fetches the actor's
/// `(&mut Magazine, &mut Tu, &LifeState)`, gates on alive + affordable, then spends the
/// magazine's own `reload_tu` and refills it to full. The actor is a Bevy [`Entity`]
/// handle — framework plumbing, the only bare type the no-bare-types rule permits in a
/// payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReloadRequested {
    /// The acting ganger whose [`Magazine`](crate::magazine::Magazine) is reloaded.
    pub actor: Entity,
}

impl ReloadRequested {
    /// Build a reload request for `actor`.
    #[must_use]
    pub const fn new(actor: Entity) -> Self {
        Self { actor }
    }
}

/// An **end-turn** act was requested — the active team passes control to the other team
/// (GTW-309).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying **NO
/// payload**. End-turn is a GLOBAL turn signal, not a per-ganger act: which team's turn is
/// ending is tracked by the [`ActiveFaction`](crate::turn::ActiveFaction) resource, NOT a
/// message field — so this is deliberately FIELDLESS (no `{ actor: Entity }`), unlike the
/// per-ganger [`ReloadRequested`] / [`MoveRequested`]. A fieldless unit struct carries no
/// domain value, so the no-bare-types rule — which wraps *values* — does not apply; the
/// type's identity IS the signal. [`dispatch_end_turn`](crate::turn::dispatch_end_turn)
/// drains this and advances the turn cycle: it hands the turn to the other team (running
/// that team's turn-start TU regen) and STOPS there (GTW-70 removed the auto-pass). The
/// enemy turn is then driven by the GTW-70 enemy-AI brain
/// ([`enemy_ai_turn`](crate::ai::enemy_ai_turn)), which emits its OWN `EndTurnRequested` to
/// hand control back to the player.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndTurnRequested;

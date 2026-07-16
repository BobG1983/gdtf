//! [`NetEvent`] — the curated combat-event vocabulary + its payload enums (GTW-734).

use serde::{Deserialize, Serialize};

use crate::{
    ids::{CellLevelNet, CellNet, GangerToken},
    view::{BodyPartNet, FactionNet, InjuryNameNet, SeverityNet},
};

/// What a fired round **impacted** — the wire mirror of the sim `ShotKind` (its entity
/// / ledger payloads dropped to a plain tag).
///
/// A QA client branches an assertion on the impact class. An independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShotKindNet {
    /// The round struck a ganger.
    Ganger,
    /// The round struck a piece of cover.
    Cover,
    /// The round was stopped by a floor / roof slab.
    Slab,
    /// The round left the grid bottom and struck the ground.
    Ground,
    /// A clean miss.
    Miss,
}

/// Why a move was **rejected** — the wire mirror of the sim `MoveRejection`.
///
/// The three no-step outcomes; an independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveRejectionNet {
    /// No route to the destination exists (the sim's `Unreachable`).
    Unreachable,
    /// A route exists but the mover cannot afford it (the sim's `Unaffordable`).
    Unaffordable,
    /// The mover is suppressed and the destination is illegal for a pinned unit (the
    /// sim's `Suppressed`).
    Suppressed,
}

/// One QA-relevant combat event — the curated subset of the sim's message vocabulary a
/// client polls (GTW-734).
///
/// Each variant mirrors a sim message worth asserting on: [`ShotFired`](Self::ShotFired)
/// (the sim `ShotFired`), [`MoveCompleted`](Self::MoveCompleted) (the sim
/// `MovementOccurred`), [`MoveRejected`](Self::MoveRejected) (the sim `MoveRejected`),
/// [`Injury`](Self::Injury) (the sim `InjuryInflicted`), [`Death`](Self::Death) (the sim
/// terminal `OnDeathOccurred`), and [`TurnStarted`](Self::TurnStarted) (the sim
/// `TurnStarted`). [`Downed`](Self::Downed) has NO dedicated sim message — a downing is
/// a `LifeState::Downed` transition — so the game side (T6) sources it from the same
/// change-detection GTW-695 uses; it is carried here because the contract lists "down"
/// as a QA-relevant event. Entity refs are wire [`GangerToken`]s (never a raw entity).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetEvent {
    /// A round was fired — its shooter, impact cell, and what it struck.
    ShotFired {
        /// The firing ganger.
        shooter: GangerToken,
        /// The `(cell, level)` the round impacted.
        impact:  CellLevelNet,
        /// What the round struck.
        kind:    ShotKindNet,
    },
    /// A ganger completed one walk step.
    MoveCompleted {
        /// The ganger that stepped.
        actor: GangerToken,
        /// The cell it stepped from.
        from:  CellNet,
        /// The cell it stepped to.
        to:    CellNet,
    },
    /// A ganger's move was rejected.
    MoveRejected {
        /// The ganger whose move was rejected.
        actor:  GangerToken,
        /// Why the move was rejected.
        reason: MoveRejectionNet,
    },
    /// A ganger went down (HP gone, still alive) — sourced from the `LifeState::Downed`
    /// transition (no dedicated sim message).
    Downed {
        /// The ganger that went down.
        entity: GangerToken,
        /// The `(cell, level)` it went down at.
        at:     CellLevelNet,
    },
    /// A ganger took a durable injury.
    Injury {
        /// The wounded ganger.
        target:   GangerToken,
        /// The injury's display name.
        name:     InjuryNameNet,
        /// The struck body part.
        part:     BodyPartNet,
        /// The rolled severity.
        severity: SeverityNet,
    },
    /// A ganger died (Wounds gone) — the terminal death gate.
    Death {
        /// The ganger that died.
        entity: GangerToken,
        /// The `(cell, level)` the death happened at.
        at:     CellLevelNet,
    },
    /// The turn passed to a faction.
    TurnStarted {
        /// The faction whose turn just started.
        now_active: FactionNet,
    },
}

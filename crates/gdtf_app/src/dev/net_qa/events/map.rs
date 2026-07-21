//! The pure act-log-to-wire event projection (GTW-739, the T6 outbox path).
//!
//! ONE curation, in one place: [`net_event_for`] projects a single sim-owned
//! [`ActEntry`] onto the QA wire's curated [`NetEvent`] vocabulary, and the [`Option`] IS
//! the curation — a deed the QA client has no use for maps to [`None`]. Built to the
//! discipline the T5 view mappers state (`net_qa/snapshot/map.rs`): every helper is a
//! pure, RNG-free, EXHAUSTIVE, WILDCARD-FREE `match`, so adding an [`ActDeed`] variant
//! forces an arm here and a new deed can never silently fall through to [`None`].
//!
//! This is the SINGLE projection over the GTW-727 act log, not a second event feed: the
//! log is the one ordered source, and this file only re-labels its entries for the wire.

use gdtf_battle_sim::{
    act_log::{ActDeed, ActEntry, PositionFacts},
    acts::MoveRejection,
    armor::BodyPart,
    ganger::{Faction, LifeState},
    metric::{Cell, CellLevel},
    resolve_coarse::ShotKind,
    severity::Severity,
};
use gdtf_qa_protocol::{
    events::{MoveRejectionNet, NetEvent, ShotKindNet},
    ids::{CellLevelNet, CellNet, CellXNet, CellYNet, GangerToken, LevelNet},
    view::{BodyPartNet, FactionNet, InjuryNameNet, SeverityNet},
};

/// Project ONE act-log [`ActEntry`] onto the curated QA wire [`NetEvent`], or [`None`]
/// when the deed is not part of the QA-relevant vocabulary (GTW-739).
///
/// The exhaustive, wildcard-free `match` below IS the outbox's curation: the mapped deeds
/// become the events a QA client polls (a fired round, a completed / rejected step, an
/// injury, a downing, a death, a turn boundary); every other deed — the drawn-state
/// snapshots, the transient-FX facts, the fire declaration whose rounds carry the shots —
/// maps to [`None`] and never reaches the wire. Adding an [`ActDeed`] variant breaks this
/// `match` at compile time, which is exactly the guarantee that a new sim act cannot slip
/// onto (or silently off) the wire unnoticed.
pub(super) fn net_event_for(entry: &ActEntry) -> Option<NetEvent> {
    let actor = GangerToken::new(entry.actor().to_bits());
    match entry.deed() {
        ActDeed::TurnBegan { now_active } => Some(NetEvent::TurnStarted {
            now_active: faction_net(*now_active),
        }),
        ActDeed::Stepped { from, to, .. } => Some(NetEvent::MoveCompleted {
            actor,
            from: cell_net(*from),
            to: cell_net(*to),
        }),
        ActDeed::MoveRefused { reason } => Some(NetEvent::MoveRejected {
            actor,
            reason: move_rejection_net(*reason),
        }),
        ActDeed::RoundResolved { shot } => Some(NetEvent::ShotFired {
            shooter: GangerToken::new(shot.shooter.to_bits()),
            impact:  cell_level_net(CellLevel::new(shot.impact_cell, shot.impact_level)),
            kind:    shot_kind_net(shot.kind),
        }),
        ActDeed::Injured { injury } => Some(NetEvent::Injury {
            target:   GangerToken::new(injury.target.to_bits()),
            name:     InjuryNameNet::new((*injury.name).clone()),
            part:     body_part_net(injury.part),
            severity: severity_net(injury.severity),
        }),
        ActDeed::LifeChanged { to, at, .. } => life_event(actor, *to, *at),
        // Not part of the QA wire vocabulary: the drawn-state snapshots the presenter
        // cursor applies, the fire DECLARATION (its rounds carry the shots), the reload /
        // magazine facts, the fall / melee / suppression / armor / bleed / DOT / field /
        // cover / throw FX facts, and the on-death CONSEQUENCE marker (the death itself
        // rides the `LifeChanged` transition above, which also excludes destroyed cover).
        ActDeed::PostureChanged { .. }
        | ActDeed::MovedTo { .. }
        | ActDeed::Fired { .. }
        | ActDeed::Reloaded { .. }
        | ActDeed::MagazineChanged { .. }
        | ActDeed::VitalsChanged { .. }
        | ActDeed::Fell { .. }
        | ActDeed::Struck { .. }
        | ActDeed::DiedAt { .. }
        | ActDeed::Suppressed { .. }
        | ActDeed::ArmorBroke { .. }
        | ActDeed::DotStarted { .. }
        | ActDeed::FieldStarted { .. }
        | ActDeed::BleedStarted
        | ActDeed::Bled
        | ActDeed::CoverSmashed { .. }
        | ActDeed::MeleeLanded { .. }
        | ActDeed::ThrowLanded { .. } => None,
    }
}

/// Project a life-state transition onto its positioned wire event: a fresh downing becomes
/// a [`Downed`](NetEvent::Downed), a death becomes a [`Death`](NetEvent::Death), and any
/// other edge (a revive, or a redundant same-state note) has no wire event.
fn life_event(actor: GangerToken, to: LifeState, at: PositionFacts) -> Option<NetEvent> {
    let at = cell_level_net(**at);
    match to {
        LifeState::Downed => Some(NetEvent::Downed { entity: actor, at }),
        LifeState::Dead => Some(NetEvent::Death { entity: actor, at }),
        LifeState::Alive => None,
    }
}

/// Mirror a sim [`Faction`] gang index onto its wire [`FactionNet`].
fn faction_net(faction: Faction) -> FactionNet {
    FactionNet::new(*faction)
}

/// Mirror a sim ground [`Cell`] onto its wire [`CellNet`].
fn cell_net(cell: Cell) -> CellNet {
    CellNet::new(CellXNet::new(cell.x), CellYNet::new(cell.y))
}

/// Mirror a sim [`CellLevel`] `(cell, level)` key onto its wire [`CellLevelNet`].
fn cell_level_net(at: CellLevel) -> CellLevelNet {
    let (cell, level) = at.split();
    CellLevelNet::new(cell_net(cell), LevelNet::new(*level))
}

/// Mirror what a fired round struck ([`ShotKind`]) onto its wire [`ShotKindNet`] — the
/// entity / ledger / cell payloads collapse to a plain impact tag.
const fn shot_kind_net(kind: ShotKind) -> ShotKindNet {
    match kind {
        ShotKind::Ganger(_) => ShotKindNet::Ganger,
        ShotKind::Cover(_) => ShotKindNet::Cover,
        ShotKind::Slab(_) => ShotKindNet::Slab,
        ShotKind::Ground(_) => ShotKindNet::Ground,
        ShotKind::Miss => ShotKindNet::Miss,
    }
}

/// Mirror a sim [`MoveRejection`] reason onto its wire [`MoveRejectionNet`].
const fn move_rejection_net(reason: MoveRejection) -> MoveRejectionNet {
    match reason {
        MoveRejection::Unreachable => MoveRejectionNet::Unreachable,
        MoveRejection::Unaffordable => MoveRejectionNet::Unaffordable,
        MoveRejection::Suppressed => MoveRejectionNet::Suppressed,
    }
}

/// Mirror a sim [`BodyPart`] onto its wire [`BodyPartNet`].
const fn body_part_net(part: BodyPart) -> BodyPartNet {
    match part {
        BodyPart::Head => BodyPartNet::Head,
        BodyPart::Torso => BodyPartNet::Torso,
        BodyPart::LeftArm => BodyPartNet::LeftArm,
        BodyPart::RightArm => BodyPartNet::RightArm,
        BodyPart::LeftLeg => BodyPartNet::LeftLeg,
        BodyPart::RightLeg => BodyPartNet::RightLeg,
    }
}

/// Mirror a sim [`Severity`] bucket onto its wire [`SeverityNet`].
const fn severity_net(severity: Severity) -> SeverityNet {
    match severity {
        Severity::None => SeverityNet::None,
        Severity::Minor => SeverityNet::Minor,
        Severity::Major => SeverityNet::Major,
        Severity::Critical => SeverityNet::Critical,
        Severity::Fatal => SeverityNet::Fatal,
    }
}

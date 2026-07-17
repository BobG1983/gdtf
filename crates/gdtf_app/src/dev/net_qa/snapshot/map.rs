//! The pure sim-to-wire value mappers (GTW-738, the T5 view path).
//!
//! One free function per sim value/enum → its `gdtf_qa_protocol` wire mirror: the
//! coordinate ([`cell_level_net`]), the faction scalar ([`faction_net`]), and the four
//! closed domain enums ([`facing_net`] / [`stance_net`] / [`life_net`] / [`severity_net`]
//! / [`body_part_net`]). Each is an exhaustive, wildcard-free `match` (adding a sim variant
//! forces an arm here), pure and RNG-free — never a leak of a sim type onto the wire.

use gdtf_battle_sim::{
    armor::BodyPart,
    ganger::{Direction, Faction, LifeState, StanceKind},
    metric::CellLevel,
    severity::Severity,
};
use gdtf_qa_protocol::{
    ids::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    intent::{FacingNet, StanceNet},
    view::{BodyPartNet, FactionNet, LifeStateNet, SeverityNet},
};

/// Mirror a sim [`Faction`] gang index onto its wire [`FactionNet`].
pub(super) fn faction_net(faction: Faction) -> FactionNet {
    FactionNet::new(*faction)
}

/// Mirror a sim [`CellLevel`] `(cell, level)` key onto its wire [`CellLevelNet`].
pub(super) fn cell_level_net(at: CellLevel) -> CellLevelNet {
    let (cell, level) = at.split();
    CellLevelNet::new(
        CellNet::new(CellXNet::new(cell.x), CellYNet::new(cell.y)),
        LevelNet::new(*level),
    )
}

/// Mirror a sim 8-way [`Direction`] onto its wire [`FacingNet`].
pub(super) const fn facing_net(direction: Direction) -> FacingNet {
    match direction {
        Direction::North => FacingNet::North,
        Direction::NorthEast => FacingNet::NorthEast,
        Direction::East => FacingNet::East,
        Direction::SouthEast => FacingNet::SouthEast,
        Direction::South => FacingNet::South,
        Direction::SouthWest => FacingNet::SouthWest,
        Direction::West => FacingNet::West,
        Direction::NorthWest => FacingNet::NorthWest,
    }
}

/// Mirror a sim [`StanceKind`] posture onto its wire [`StanceNet`].
pub(super) const fn stance_net(stance: StanceKind) -> StanceNet {
    match stance {
        StanceKind::Standing => StanceNet::Standing,
        StanceKind::Crouching => StanceNet::Crouching,
        StanceKind::Prone => StanceNet::Prone,
    }
}

/// Mirror a sim [`LifeState`] onto its wire [`LifeStateNet`].
pub(super) const fn life_net(life: LifeState) -> LifeStateNet {
    match life {
        LifeState::Alive => LifeStateNet::Alive,
        LifeState::Downed => LifeStateNet::Downed,
        LifeState::Dead => LifeStateNet::Dead,
    }
}

/// Mirror a sim [`Severity`] bucket onto its wire [`SeverityNet`].
pub(super) const fn severity_net(severity: Severity) -> SeverityNet {
    match severity {
        Severity::None => SeverityNet::None,
        Severity::Minor => SeverityNet::Minor,
        Severity::Major => SeverityNet::Major,
        Severity::Critical => SeverityNet::Critical,
        Severity::Fatal => SeverityNet::Fatal,
    }
}

/// Mirror a sim [`BodyPart`] onto its wire [`BodyPartNet`].
pub(super) const fn body_part_net(part: BodyPart) -> BodyPartNet {
    match part {
        BodyPart::Head => BodyPartNet::Head,
        BodyPart::Torso => BodyPartNet::Torso,
        BodyPart::LeftArm => BodyPartNet::LeftArm,
        BodyPart::RightArm => BodyPartNet::RightArm,
        BodyPart::LeftLeg => BodyPartNet::LeftLeg,
        BodyPart::RightLeg => BodyPartNet::RightLeg,
    }
}

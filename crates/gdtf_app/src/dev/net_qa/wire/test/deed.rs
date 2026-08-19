use bevy::ecs::entity::Entity;
use gdtf_battle_sim::{
    act_log::{ActDeed, MagazineFacts, PoseFacts, PositionFacts, SuppressedNow, VitalsFacts},
    acts::{InjuryInflicted, MoveRejection, ReloadOutcome, RoundCount},
    armor::BodyPart,
    effects::fields::FieldDamage,
    entity::TerrainPieceKind,
    falls::StoreysFallen,
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds,
    },
    inflicted_wound::InflictedWounds,
    injuries::{GainedInjury, InflictedInjuries, InjuryName, InspectText, LogText, PopupText},
    magazine::{Magazine, ReloadTu},
    metric::SimPos,
    prelude::{Cell, CellLevel, Level},
    resolve_coarse::ShotKind,
    resolve_hit::HpDamage,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    weapon::{DamageType, DotDamage, MagazineSize, ModeKind},
};

use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::deed::{
    ActDeedKindNet, MoveRejectionNet, ReloadOutcomeNet, TerrainPieceKindNet,
};

const KINDS: [ActDeedKindNet; 29] = [
    ActDeedKindNet::TurnBegan,
    ActDeedKindNet::PostureChanged,
    ActDeedKindNet::Stepped,
    ActDeedKindNet::MovedTo,
    ActDeedKindNet::MoveRefused {
        reason: MoveRejectionNet::Unreachable,
    },
    ActDeedKindNet::Fired,
    ActDeedKindNet::RoundResolved,
    ActDeedKindNet::Reloaded {
        outcome: ReloadOutcomeNet::AlreadyFull,
    },
    ActDeedKindNet::MagazineChanged,
    ActDeedKindNet::Injured,
    ActDeedKindNet::VitalsChanged,
    ActDeedKindNet::Fell,
    ActDeedKindNet::Struck,
    ActDeedKindNet::DiedAt,
    ActDeedKindNet::Suppressed,
    ActDeedKindNet::ArmorBroke,
    ActDeedKindNet::DotStarted,
    ActDeedKindNet::FieldStarted,
    ActDeedKindNet::BleedStarted,
    ActDeedKindNet::Bled,
    ActDeedKindNet::DotTicked,
    ActDeedKindNet::FieldTicked,
    ActDeedKindNet::TerrainPieceSmashed {
        kind: TerrainPieceKindNet::Wall,
    },
    ActDeedKindNet::TerrainPieceSmashed {
        kind: TerrainPieceKindNet::Cover,
    },
    ActDeedKindNet::TerrainPieceSmashed {
        kind: TerrainPieceKindNet::Slab,
    },
    ActDeedKindNet::TerrainPieceSmashed {
        kind: TerrainPieceKindNet::Emplacement,
    },
    ActDeedKindNet::MeleeLanded,
    ActDeedKindNet::ThrowLanded,
    ActDeedKindNet::LifeChanged,
];

fn an_entity() -> Entity {
    Entity::from_raw_u32(7).unwrap_or(Entity::PLACEHOLDER)
}

fn a_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 3), Level::new(0))
}

fn a_position() -> PositionFacts {
    PositionFacts::new(Position::new(a_cell()))
}

fn a_shot() -> ShotFired {
    ShotFired {
        shooter:      an_entity(),
        muzzle:       SimPos::new(0.0, 0.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::X),
        impact_cell:  Cell::new(2, 3),
        impact_level: Level::new(0),
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       None,
    }
}

fn an_injury() -> InjuryInflicted {
    InjuryInflicted {
        target:       an_entity(),
        gained:       GainedInjury::new(
            InjuryName::new("cracked_rib".to_owned()),
            BodyPart::Torso,
            Severity::Major,
            Vec::new(),
            InspectText::new("aches".to_owned()),
        ),
        name:         InjuryName::new("cracked_rib".to_owned()),
        part:         BodyPart::Torso,
        severity:     Severity::Major,
        popup_text:   PopupText::new("rib!".to_owned()),
        log_text:     LogText::new("a rib cracks".to_owned()),
        inspect_text: InspectText::new("aches".to_owned()),
    }
}

/// The sim reason a wire reason came from. Exhaustive, so a crossed arm shows here.
const fn a_sim_rejection(reason: MoveRejectionNet) -> MoveRejection {
    match reason {
        MoveRejectionNet::Unreachable => MoveRejection::Unreachable,
        MoveRejectionNet::Unaffordable => MoveRejection::Unaffordable,
        MoveRejectionNet::Suppressed => MoveRejection::Suppressed,
    }
}

/// The sim outcome a wire outcome came from. Exhaustive, so a crossed arm shows here.
const fn a_sim_outcome(outcome: ReloadOutcomeNet) -> ReloadOutcome {
    match outcome {
        ReloadOutcomeNet::Reloaded => ReloadOutcome::Reloaded,
        ReloadOutcomeNet::AlreadyFull => ReloadOutcome::AlreadyFull,
        ReloadOutcomeNet::NoTu => ReloadOutcome::NoTu,
    }
}

/// The sim kind a wire kind came from. Exhaustive, so a crossed arm shows here.
const fn a_sim_piece_kind(kind: TerrainPieceKindNet) -> TerrainPieceKind {
    match kind {
        TerrainPieceKindNet::Wall => TerrainPieceKind::Wall,
        TerrainPieceKindNet::Cover => TerrainPieceKind::Cover,
        TerrainPieceKindNet::Slab => TerrainPieceKind::Slab,
        TerrainPieceKindNet::Emplacement => TerrainPieceKind::Emplacement,
    }
}

/// One sim deed per wire kind. Exhaustive, so a new kind needs a witness before this compiles.
fn a_deed_for(kind: ActDeedKindNet) -> ActDeed {
    match kind {
        ActDeedKindNet::TurnBegan => ActDeed::TurnBegan {
            now_active: Faction::new(1),
        },
        ActDeedKindNet::PostureChanged => ActDeed::PostureChanged {
            pose: PoseFacts::new(
                Facing::new(Direction::North),
                Stance::new(StanceKind::Crouching),
                Aiming::new(true),
                SuppressedNow::new(false),
            ),
        },
        ActDeedKindNet::Stepped => ActDeed::Stepped {
            from:     Cell::new(1, 1),
            to:       Cell::new(2, 3),
            position: a_position(),
        },
        ActDeedKindNet::MovedTo => ActDeed::MovedTo {
            position: a_position(),
        },
        ActDeedKindNet::MoveRefused { reason } => ActDeed::MoveRefused {
            reason: a_sim_rejection(reason),
        },
        ActDeedKindNet::Fired => ActDeed::Fired {
            target: Some(an_entity()),
            mode:   ModeKind::Burst,
            rounds: RoundCount::new(3),
        },
        ActDeedKindNet::RoundResolved => ActDeed::RoundResolved {
            shot: Box::new(a_shot()),
        },
        ActDeedKindNet::Reloaded { outcome } => ActDeed::Reloaded {
            outcome: a_sim_outcome(outcome),
        },
        ActDeedKindNet::MagazineChanged => ActDeed::MagazineChanged {
            magazine: MagazineFacts::new(Magazine::loaded(MagazineSize::new(12), ReloadTu::new(4))),
        },
        ActDeedKindNet::Injured => ActDeed::Injured {
            injury: Box::new(an_injury()),
        },
        ActDeedKindNet::VitalsChanged => ActDeed::VitalsChanged {
            vitals: VitalsFacts::new(
                Tu::new(10),
                Hp::new(8),
                Wounds::new(2),
                InflictedWounds::default(),
                InflictedInjuries::default(),
            ),
        },
        ActDeedKindNet::Fell => ActDeed::Fell {
            from_level: Level::new(2),
            to_level:   Level::new(0),
            storeys:    StoreysFallen::new(2),
        },
        ActDeedKindNet::Struck => ActDeed::Struck {
            target:    an_entity(),
            hp_damage: HpDamage::new(5),
        },
        ActDeedKindNet::DiedAt => ActDeed::DiedAt { at: a_cell() },
        ActDeedKindNet::Suppressed => ActDeed::Suppressed { at: a_cell() },
        ActDeedKindNet::ArmorBroke => ActDeed::ArmorBroke {
            part: BodyPart::Head,
        },
        ActDeedKindNet::DotStarted => ActDeed::DotStarted {
            per_turn: DotDamage::new(3),
        },
        ActDeedKindNet::FieldStarted => ActDeed::FieldStarted { at: a_cell() },
        ActDeedKindNet::BleedStarted => ActDeed::BleedStarted,
        ActDeedKindNet::Bled => ActDeed::Bled,
        ActDeedKindNet::DotTicked => ActDeed::DotTicked {
            at:     a_cell(),
            amount: DotDamage::new(1),
        },
        ActDeedKindNet::FieldTicked => ActDeed::FieldTicked {
            at:     a_cell(),
            amount: FieldDamage::new(2),
        },
        ActDeedKindNet::TerrainPieceSmashed { kind } => ActDeed::TerrainPieceSmashed {
            at:   a_cell(),
            kind: a_sim_piece_kind(kind),
        },
        ActDeedKindNet::MeleeLanded => ActDeed::MeleeLanded {
            at:     a_cell(),
            damage: DamageType::Rend,
        },
        ActDeedKindNet::ThrowLanded => ActDeed::ThrowLanded {
            at:     a_cell(),
            damage: DamageType::Blast,
        },
        ActDeedKindNet::LifeChanged => ActDeed::LifeChanged {
            from: LifeState::Alive,
            to:   LifeState::Downed,
            at:   a_position(),
        },
    }
}

/// The variant name at the head of a `Debug` rendering, without its payload.
fn variant_name(rendered: &str) -> &str {
    rendered.split([' ', '{', '(']).next().unwrap_or(rendered)
}

#[test]
fn every_deed_kind_round_trips() {
    for kind in KINDS {
        assert_ron_round_trip(&kind);
    }
}

#[test]
fn every_move_rejection_round_trips_and_mirrors_its_sim_reason() {
    for reason in [
        MoveRejectionNet::Unreachable,
        MoveRejectionNet::Unaffordable,
        MoveRejectionNet::Suppressed,
    ] {
        assert_ron_round_trip(&reason);
        assert_eq!(
            MoveRejectionNet::from_sim(a_sim_rejection(reason)),
            reason,
            "a refused move carries the sim's own reason to the client, so the mirror must be \
             lossless both ways",
        );
    }
}

#[test]
fn every_sim_deed_mirrors_onto_the_kind_of_the_same_name() {
    for kind in KINDS {
        let deed = a_deed_for(kind);
        assert_eq!(
            ActDeedKindNet::from_deed(&deed),
            kind,
            "`{deed:?}` must mirror onto `{kind:?}`",
        );
        assert_eq!(
            variant_name(&format!("{kind:?}")),
            variant_name(&format!("{deed:?}")),
            "each kind carries its sim deed's own name, so a crossed arm shows here",
        );
    }
}

#[test]
fn every_reload_outcome_round_trips_and_mirrors_its_sim_outcome() {
    for outcome in [
        ReloadOutcomeNet::Reloaded,
        ReloadOutcomeNet::AlreadyFull,
        ReloadOutcomeNet::NoTu,
    ] {
        assert_ron_round_trip(&outcome);
        assert_eq!(
            ReloadOutcomeNet::from_sim(a_sim_outcome(outcome)),
            outcome,
            "a logged reload carries the sim's outcome, so the mirror is lossless both ways",
        );
        assert_eq!(
            ActDeedKindNet::from_deed(&ActDeed::Reloaded {
                outcome: a_sim_outcome(outcome),
            }),
            ActDeedKindNet::Reloaded { outcome },
            "the sim logs a reload it declined and one it made alike, so the kind has to carry \
             the outcome for a client to tell them apart",
        );
    }
}

#[test]
fn every_terrain_piece_kind_round_trips_and_mirrors_its_sim_kind() {
    for kind in TerrainPieceKind::ALL {
        let wire = TerrainPieceKindNet::from_sim(kind);
        assert_ron_round_trip(&wire);
        assert_eq!(
            ActDeedKindNet::from_deed(&ActDeed::TerrainPieceSmashed { at: a_cell(), kind }),
            ActDeedKindNet::TerrainPieceSmashed { kind: wire },
            "a smash tells a client which kind of piece went down, so the kind has to carry it \
             — mirroring `{kind:?}` gave `{:?}`",
            ActDeedKindNet::from_deed(&ActDeed::TerrainPieceSmashed { at: a_cell(), kind }),
        );
    }
}

#[test]
fn the_kind_list_has_no_duplicates() {
    let mut seen: Vec<String> = KINDS.iter().map(|kind| format!("{kind:?}")).collect();
    seen.sort_unstable();
    let total = seen.len();
    seen.dedup();
    assert_eq!(
        seen.len(),
        total,
        "each sim deed gets its own kind, so a new sim variant fails to compile until it \
         gains a mirror",
    );
}

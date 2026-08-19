use gdtf_battle_sim::act_log::{ActDeed, ActLog, ActProvenance, ActSeq, RecordedAct};

use super::harness::deed_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Disposition {
    Replayed,
    DrawnOnly,
}

const fn disposition(deed: &ActDeed) -> Disposition {
    match *deed {
        ActDeed::TurnBegan { .. }
        | ActDeed::Stepped { .. }
        | ActDeed::MoveRefused { .. }
        | ActDeed::Fired { .. }
        | ActDeed::RoundResolved { .. }
        | ActDeed::Reloaded { .. }
        | ActDeed::Injured { .. }
        | ActDeed::Fell { .. }
        | ActDeed::Struck { .. }
        | ActDeed::DiedAt { .. }
        | ActDeed::Suppressed { .. }
        | ActDeed::ArmorBroke { .. }
        | ActDeed::DotStarted { .. }
        | ActDeed::FieldStarted { .. }
        | ActDeed::BleedStarted
        | ActDeed::Bled
        | ActDeed::DotTicked { .. }
        | ActDeed::FieldTicked { .. }
        | ActDeed::TerrainPieceSmashed { .. }
        | ActDeed::MeleeLanded { .. }
        | ActDeed::ThrowLanded { .. } => Disposition::Replayed,
        ActDeed::PostureChanged { .. }
        | ActDeed::MovedTo { .. }
        | ActDeed::MagazineChanged { .. }
        | ActDeed::VitalsChanged { .. }
        | ActDeed::LifeChanged { .. } => Disposition::DrawnOnly,
    }
}

#[test]
fn every_deed_maps_or_is_deliberately_unmapped() {
    let samples = sample_deeds();
    assert!(
        samples.len() >= 20,
        "the sample set must cover the deed vocabulary broadly enough to be a real check",
    );

    let mut replayed = 0_usize;
    let mut drawn_only = 0_usize;
    for deed in &samples {
        match disposition(deed) {
            Disposition::Replayed => replayed += 1,
            Disposition::DrawnOnly => drawn_only += 1,
        }
    }
    assert!(
        replayed > 0 && drawn_only > 0,
        "both dispositions must be populated — {replayed} replayed, {drawn_only} drawn-only",
    );

    let mut names: Vec<&'static str> = samples.iter().map(deed_name).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(
        names.len(),
        total,
        "each deed variant must have its OWN stable name",
    );
}

#[test]
fn every_sampled_deed_round_trips_through_the_log() {
    let mut log = ActLog::default();
    let actor = bevy::prelude::Entity::PLACEHOLDER;
    let samples = sample_deeds();
    for deed in samples.clone() {
        log.append(RecordedAct::new(actor, ActProvenance::Clock, deed));
    }

    let read: Vec<&ActDeed> = log
        .since(ActSeq::START)
        .map(gdtf_battle_sim::act_log::ActEntry::deed)
        .collect();
    let expected: Vec<&ActDeed> = samples.iter().collect();
    assert_eq!(
        read, expected,
        "the log stores and returns every deed unchanged, in append order",
    );
}

use gdtf_battle_sim::{
    act_log::{MagazineFacts, PoseFacts, PositionFacts, SuppressedNow, VitalsFacts},
    acts::{MoveRejection, ReloadOutcome, RoundCount},
    armor::BodyPart,
    effects::fields::FieldDamage,
    entity::TerrainPieceKind,
    falls::StoreysFallen,
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds,
    },
    magazine::Magazine,
    metric::{Cell, CellLevel, Level},
    resolve_hit::HpDamage,
    weapon::{DamageType, DotDamage, ModeKind},
};

fn sample_deeds() -> Vec<ActDeed> {
    let cell = Cell::new(1, 1);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let pose = PoseFacts::new(
        Facing::new(Direction::North),
        Stance::new(StanceKind::Standing),
        Aiming::new(false),
        SuppressedNow::new(false),
    );
    let position = PositionFacts::new(Position::new(at));
    let vitals = VitalsFacts::new(
        Tu::new(1),
        Hp::new(1),
        Wounds::new(1),
        gdtf_battle_sim::inflicted_wound::InflictedWounds::default(),
        gdtf_battle_sim::injuries::InflictedInjuries::default(),
    );

    vec![
        ActDeed::TurnBegan {
            now_active: Faction::new(0),
        },
        ActDeed::PostureChanged { pose },
        ActDeed::Stepped {
            from: cell,
            to: cell,
            position,
        },
        ActDeed::MovedTo { position },
        ActDeed::MoveRefused {
            reason: MoveRejection::Unreachable,
        },
        ActDeed::Fired {
            target: None,
            mode:   ModeKind::Single,
            rounds: RoundCount::new(1),
        },
        ActDeed::Reloaded {
            outcome: ReloadOutcome::Reloaded,
        },
        ActDeed::MagazineChanged {
            magazine: MagazineFacts::new(Magazine::default()),
        },
        ActDeed::VitalsChanged { vitals },
        ActDeed::Fell {
            from_level: level,
            to_level:   level,
            storeys:    StoreysFallen::new(1),
        },
        ActDeed::Struck {
            target:    bevy::prelude::Entity::PLACEHOLDER,
            hp_damage: HpDamage::new(1),
        },
        ActDeed::DiedAt { at },
        ActDeed::Suppressed { at },
        ActDeed::ArmorBroke {
            part: BodyPart::Torso,
        },
        ActDeed::DotStarted {
            per_turn: DotDamage::new(1),
        },
        ActDeed::FieldStarted { at },
        ActDeed::BleedStarted,
        ActDeed::Bled,
        ActDeed::DotTicked {
            at,
            amount: DotDamage::new(1),
        },
        ActDeed::FieldTicked {
            at,
            amount: FieldDamage::new(1),
        },
        ActDeed::TerrainPieceSmashed {
            at,
            kind: TerrainPieceKind::Wall,
        },
        ActDeed::MeleeLanded {
            at,
            damage: DamageType::Kinetic,
        },
        ActDeed::ThrowLanded {
            at,
            damage: DamageType::Kinetic,
        },
        ActDeed::LifeChanged {
            from: LifeState::Alive,
            to:   LifeState::Downed,
            at:   position,
        },
    ]
}

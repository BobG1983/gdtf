//! T8 — every deed is a CONSCIOUS decision, forced at compile time.

use gdtf_battle_sim::act_log::{ActDeed, ActLog, ActProvenance, ActSeq, RecordedAct};

use super::harness::deed_name;

/// Whether a deed is meant to reach a downstream consumer as a shown fact, or is
/// deliberately silent there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Disposition {
    /// The deed re-emits a sim fact when it is shown (a combat-log line, an FX flash, a
    /// bolt).
    Replayed,
    /// The deed writes DRAWN state only — there is no message to re-emit, because the sim
    /// never had one.
    DrawnOnly,
}

/// The wildcard-free disposition map.
///
/// This is the compile-time forcing function C4's completeness bar rests on: there is no
/// compile error for a MISSING deed, but there IS one for an UNCLASSIFIED deed. A new
/// variant added to [`ActDeed`] fails to compile here until somebody states, in one place,
/// whether it is something the player is shown or purely drawn state — which is exactly
/// the decision that is easy to forget and expensive to get wrong (a deed with no
/// disposition would silently show live state instead of cursor-time state).
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
        | ActDeed::CoverSmashed { .. }
        | ActDeed::MeleeLanded { .. }
        | ActDeed::ThrowLanded { .. } => Disposition::Replayed,
        // The four query-sourced state snapshots: the sim has no message for any of them,
        // so showing one means writing the drawn mirror, not re-emitting a fact.
        ActDeed::PostureChanged { .. }
        | ActDeed::MovedTo { .. }
        | ActDeed::MagazineChanged { .. }
        | ActDeed::VitalsChanged { .. }
        | ActDeed::LifeChanged { .. } => Disposition::DrawnOnly,
    }
}

/// **T8 — a new deed forces a conscious decision.** Every deed the log can hold is
/// classified, and the classification is exhaustive over the enum.
///
/// The real guarantee here is the `match` above, which the compiler checks. This test
/// additionally proves the classifier is total over live values and that both dispositions
/// are actually populated — a map that had collapsed to one arm would still compile.
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

    // Names are unique, so an assertion that filters by name cannot silently match two
    // different variants.
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

/// Appending every sampled deed round-trips through the log unchanged, so the log itself
/// is agnostic to the vocabulary it carries.
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

/// One sample of each deed variant, built from cheap defaults.
fn sample_deeds() -> Vec<ActDeed> {
    use gdtf_battle_sim::{
        act_log::{MagazineFacts, PoseFacts, PositionFacts, SuppressedNow, VitalsFacts},
        acts::{MoveRejection, ReloadOutcome, RoundCount},
        armor::BodyPart,
        falls::StoreysFallen,
        ganger::{
            Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu,
            Wounds,
        },
        magazine::Magazine,
        metric::{Cell, CellLevel, Level},
        resolve_hit::HpDamage,
        weapon::{DamageType, DotDamage, ModeKind},
    };

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
        ActDeed::CoverSmashed { at },
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

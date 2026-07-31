//! Unit coverage for the T6 projection (GTW-739): the exhaustiveness guard on
//! [`net_event_for`](super::map::net_event_for).
//!
//! The projection's OWN wildcard-free `match` is the compile guard — a new
//! [`ActDeed`](gdtf_battle_sim::act_log::ActDeed) variant breaks it. [`is_wire_event`]
//! below is the belt-and-suspenders twin: its exhaustive classifier breaks on the same new
//! variant, and [`net_event_for_agrees_with_the_wildcard_free_classifier`] proves the two
//! stay in step for every deed it can cheaply build.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActLogCapacity, ActProvenance, PositionFacts, RecordedAct},
    acts::MoveRejection,
    effects::fields::FieldDamage,
    ganger::{LifeState, Position},
    metric::{Cell, CellLevel, Level},
    weapon::{DamageType, DotDamage},
};

use super::map::net_event_for;

/// Whether `deed` belongs to the curated wire vocabulary — an exhaustive, wildcard-free
/// classifier mirroring [`net_event_for`](super::map::net_event_for), so a NEW `ActDeed`
/// variant fails to compile HERE as well as in the projection it guards.
fn is_wire_event(deed: &ActDeed) -> bool {
    match deed {
        ActDeed::TurnBegan { .. }
        | ActDeed::Stepped { .. }
        | ActDeed::MoveRefused { .. }
        | ActDeed::RoundResolved { .. }
        | ActDeed::Injured { .. } => true,
        // A downing or a death is a wire event; a revive (or any edge into `Alive`) is not.
        ActDeed::LifeChanged { to, .. } => !matches!(to, LifeState::Alive),
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
        | ActDeed::DotTicked { .. }
        | ActDeed::FieldTicked { .. }
        | ActDeed::CoverSmashed { .. }
        | ActDeed::MeleeLanded { .. }
        | ActDeed::ThrowLanded { .. } => false,
    }
}

/// A ground-level [`PositionFacts`] at cell `(x, y)`.
fn pos(x: i32, y: i32) -> PositionFacts {
    PositionFacts::new(Position::new(CellLevel::new(
        Cell::new(x, y),
        Level::new(0),
    )))
}

/// For every cheaply-constructible deed — mapped and unmapped — [`net_event_for`] returns
/// `Some` exactly when [`is_wire_event`] says the deed is on the wire.
#[test]
fn net_event_for_agrees_with_the_wildcard_free_classifier() {
    let cell = Cell::new(2, 3);
    let at = CellLevel::new(cell, Level::new(0));
    let samples = [
        ActDeed::TurnBegan {
            now_active: gdtf_battle_sim::ganger::Faction::new(1),
        },
        ActDeed::Stepped {
            from:     cell,
            to:       Cell::new(3, 3),
            position: pos(3, 3),
        },
        ActDeed::MoveRefused {
            reason: MoveRejection::Unreachable,
        },
        ActDeed::LifeChanged {
            from: LifeState::Alive,
            to:   LifeState::Downed,
            at:   pos(2, 3),
        },
        ActDeed::LifeChanged {
            from: LifeState::Downed,
            to:   LifeState::Dead,
            at:   pos(2, 3),
        },
        ActDeed::LifeChanged {
            from: LifeState::Downed,
            to:   LifeState::Alive,
            at:   pos(2, 3),
        },
        ActDeed::Suppressed { at },
        ActDeed::BleedStarted,
        // GTW-889's two new deeds: both are drawn-only ticks, so the projection must
        // return `None` for them. Sampled here so flipping either to a wire event fails.
        ActDeed::DotTicked {
            at,
            amount: DotDamage::new(1),
        },
        ActDeed::FieldTicked {
            at,
            amount: FieldDamage::new(1),
        },
        ActDeed::MeleeLanded {
            at,
            damage: DamageType::Kinetic,
        },
    ];

    let mut log = ActLog::new(ActLogCapacity::new(64));
    for deed in samples {
        let want = is_wire_event(&deed);
        let seq = log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ActProvenance::Clock,
            deed,
        ));
        // A just-appended entry is always retained in an over-sized log.
        assert!(log.at(seq).is_some(), "an appended entry must be retained");
        let Some(entry) = log.at(seq) else { continue };
        assert_eq!(
            net_event_for(entry).is_some(),
            want,
            "net_event_for disagreed with is_wire_event for {:?}",
            entry.deed(),
        );
    }
}

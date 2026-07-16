//! Exhaustive per-variant round-trip + parity-forcing pins for [`NetEvent`] and its
//! payload enums (GTW-734).

use crate::{
    events::{DroppedCount, EventBatch, MoveRejectionNet, NetEvent, ShotKindNet},
    ids::{CellLevelNet, CellNet, CellXNet, CellYNet, GangerToken, LevelNet},
    test_support::assert_ron_round_trip,
    view::{BodyPartNet, FactionNet, InjuryNameNet, SeverityNet},
};

/// A representative cell for building event cases.
fn a_cell() -> CellNet {
    CellNet::new(CellXNet::new(1), CellYNet::new(1))
}

/// A representative `(cell, level)` key for building event cases.
fn a_key() -> CellLevelNet {
    CellLevelNet::new(a_cell(), LevelNet::new(0))
}

/// Every [`NetEvent`] variant — the round-trip table, kept in lock-step with the enum
/// by [`net_event_is_exhaustive`].
fn net_event_cases() -> Vec<NetEvent> {
    vec![
        NetEvent::ShotFired {
            shooter: GangerToken::new(1),
            impact:  a_key(),
            kind:    ShotKindNet::Ganger,
        },
        NetEvent::MoveCompleted {
            actor: GangerToken::new(2),
            from:  a_cell(),
            to:    a_cell(),
        },
        NetEvent::MoveRejected {
            actor:  GangerToken::new(3),
            reason: MoveRejectionNet::Unreachable,
        },
        NetEvent::Downed {
            entity: GangerToken::new(4),
            at:     a_key(),
        },
        NetEvent::Injury {
            target:   GangerToken::new(5),
            name:     InjuryNameNet::new("Gut Wound".to_owned()),
            part:     BodyPartNet::Torso,
            severity: SeverityNet::Critical,
        },
        NetEvent::Death {
            entity: GangerToken::new(6),
            at:     a_key(),
        },
        NetEvent::TurnStarted {
            now_active: FactionNet::new(1),
        },
    ]
}

/// The wildcard-free witness — adding a [`NetEvent`] variant breaks this `match` until
/// it (and [`net_event_cases`]) gain the new arm.
fn net_event_is_exhaustive(event: &NetEvent) {
    match event {
        NetEvent::ShotFired { .. }
        | NetEvent::MoveCompleted { .. }
        | NetEvent::MoveRejected { .. }
        | NetEvent::Downed { .. }
        | NetEvent::Injury { .. }
        | NetEvent::Death { .. }
        | NetEvent::TurnStarted { .. } => {}
    }
}

/// Every [`NetEvent`] variant round-trips through compact RON identically.
#[test]
fn net_event_round_trips_every_variant() {
    let cases = net_event_cases();
    assert_eq!(
        cases.len(),
        7,
        "the case table lists every NetEvent variant"
    );
    for case in &cases {
        net_event_is_exhaustive(case);
        assert_ron_round_trip(case);
    }
}

/// Every [`ShotKindNet`] and [`MoveRejectionNet`] variant round-trips; the witnesses
/// force new variants in.
#[test]
fn shot_kind_and_move_rejection_round_trip() {
    for kind in [
        ShotKindNet::Ganger,
        ShotKindNet::Cover,
        ShotKindNet::Slab,
        ShotKindNet::Ground,
        ShotKindNet::Miss,
    ] {
        match kind {
            ShotKindNet::Ganger
            | ShotKindNet::Cover
            | ShotKindNet::Slab
            | ShotKindNet::Ground
            | ShotKindNet::Miss => {}
        }
        assert_ron_round_trip(&kind);
    }
    for reason in [
        MoveRejectionNet::Unreachable,
        MoveRejectionNet::Unaffordable,
        MoveRejectionNet::Suppressed,
    ] {
        match reason {
            MoveRejectionNet::Unreachable
            | MoveRejectionNet::Unaffordable
            | MoveRejectionNet::Suppressed => {}
        }
        assert_ron_round_trip(&reason);
    }
}

/// An [`EventBatch`] — carrying a mix of events + a non-zero dropped count —
/// round-trips.
#[test]
fn event_batch_round_trips() {
    assert_ron_round_trip(&EventBatch::new(net_event_cases(), DroppedCount::new(3)));
    assert_ron_round_trip(&EventBatch::new(vec![], DroppedCount::new(0)));
}

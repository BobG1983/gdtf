use gdtf_battle_input::ShotRefusal;
use gdtf_battle_sim::{
    acts::ReloadOutcome,
    posture::{FacingRefusal, StanceRefusal},
};

use crate::dev::net_qa::wire::{
    act::{ActCompleteNet, ActRefusalNet, ActReply, ActSeqNet, SelectReply},
    click::{ClickDecisionNet, ClickReply},
    deed::MoveRejectionNet,
    refusal::{FacingRefusalNet, ReloadRefusalNet, ShotRefusalNet, StanceRefusalNet},
    test::assert_ron_round_trip,
    token::GangerToken,
};

fn act_refusals() -> [ActRefusalNet; 3] {
    [
        ActRefusalNet::UnknownToken,
        ActRefusalNet::NoShooter,
        ActRefusalNet::NoOffer,
    ]
}

/// Every reason a shot can be turned down, in the order the game checks them.
const SHOT_REFUSALS: [ShotRefusal; 7] = [
    ShotRefusal::NotAShooter,
    ShotRefusal::NoFiringWeapon,
    ShotRefusal::NotAlive,
    ShotRefusal::Unaffordable,
    ShotRefusal::MagazineEmpty,
    ShotRefusal::OutOfBounds,
    ShotRefusal::NotEnoughHands,
];

/// An accepted window, used wherever a case needs one that is not a refusal.
const A_WINDOW: ActReply = ActReply::Accepted {
    from_seq: ActSeqNet::new(7),
    to_seq:   ActSeqNet::new(11),
    complete: ActCompleteNet::new(false),
};

#[test]
fn act_complete_round_trips_both_ways() {
    assert_ron_round_trip(&ActCompleteNet::new(true));
    assert_ron_round_trip(&ActCompleteNet::new(false));
}

#[test]
fn act_refusal_round_trips_every_variant() {
    for reason in act_refusals() {
        match reason {
            ActRefusalNet::UnknownToken | ActRefusalNet::NoShooter | ActRefusalNet::NoOffer => {}
        }
        assert_ron_round_trip(&reason);
    }
}

#[test]
fn shot_refusal_round_trips_every_variant_and_mirrors_the_game() {
    for refusal in SHOT_REFUSALS {
        let wire = ShotRefusalNet::from_sim(refusal);
        assert_ron_round_trip(&wire);
        assert_eq!(
            format!("{wire:?}"),
            format!("{refusal:?}"),
            "a refused shot carries the game's own reason, so each mirror keeps its name",
        );
    }
}

#[test]
fn the_shot_refusals_are_all_told_apart() {
    let mut seen: Vec<String> = SHOT_REFUSALS
        .into_iter()
        .map(|refusal| format!("{:?}", ShotRefusalNet::from_sim(refusal)))
        .collect();
    seen.sort_unstable();
    let total = seen.len();
    seen.dedup();
    assert_eq!(
        seen.len(),
        total,
        "each reason a shot is turned down gets its own wire variant, so a client can tell an \
         empty magazine from a target off the grid",
    );
}

#[test]
fn reload_refusal_round_trips_and_only_a_decline_has_one() {
    for outcome in [ReloadOutcome::AlreadyFull, ReloadOutcome::NoTu] {
        let Some(wire) = ReloadRefusalNet::from_sim(outcome) else {
            unreachable!("a reload the sim declined has a reason to give: {outcome:?}");
        };
        assert_ron_round_trip(&wire);
    }
    assert_eq!(
        ReloadRefusalNet::from_sim(ReloadOutcome::Reloaded),
        None,
        "a reload that refilled the magazine is not a refusal, so it carries no reason",
    );
    assert_ne!(
        ReloadRefusalNet::from_sim(ReloadOutcome::AlreadyFull),
        ReloadRefusalNet::from_sim(ReloadOutcome::NoTu),
        "a full magazine and an empty TU pool are separate reasons on the wire",
    );
}

#[test]
fn stance_refusal_round_trips_every_variant() {
    for refusal in [StanceRefusal::AlreadyHeld, StanceRefusal::Unaffordable] {
        assert_ron_round_trip(&StanceRefusalNet::from_sim(refusal));
    }
    assert_ne!(
        StanceRefusalNet::from_sim(StanceRefusal::AlreadyHeld),
        StanceRefusalNet::from_sim(StanceRefusal::Unaffordable),
        "the stance already held and one the actor cannot pay for are separate reasons",
    );
}

#[test]
fn facing_refusal_round_trips_every_variant() {
    for refusal in [FacingRefusal::AlreadyFacing, FacingRefusal::Unaffordable] {
        assert_ron_round_trip(&FacingRefusalNet::from_sim(refusal));
    }
    assert_ne!(
        FacingRefusalNet::from_sim(FacingRefusal::AlreadyFacing),
        FacingRefusalNet::from_sim(FacingRefusal::Unaffordable),
        "the direction already faced and a turn the actor cannot pay for are separate reasons",
    );
}

#[test]
fn act_reply_round_trips_every_variant() {
    let cases = [
        A_WINDOW,
        ActReply::Refused {
            reason: ActRefusalNet::NoShooter,
        },
        ActReply::FireRefused {
            reason: ShotRefusalNet::MagazineEmpty,
        },
        ActReply::ReloadRefused {
            reason: ReloadRefusalNet::AlreadyFull,
        },
        ActReply::MoveRefused {
            reason: MoveRejectionNet::Unreachable,
        },
        ActReply::StanceRefused {
            reason: StanceRefusalNet::AlreadyHeld,
        },
        ActReply::FacingRefused {
            reason: FacingRefusalNet::AlreadyFacing,
        },
    ];
    for case in cases {
        match case {
            ActReply::Accepted { .. }
            | ActReply::Refused { .. }
            | ActReply::FireRefused { .. }
            | ActReply::ReloadRefused { .. }
            | ActReply::MoveRefused { .. }
            | ActReply::StanceRefused { .. }
            | ActReply::FacingRefused { .. } => {}
        }
        assert_ron_round_trip(&case);
    }
}

#[test]
fn a_refused_act_is_never_an_accepted_window() {
    let refusals = [
        ActReply::FireRefused {
            reason: ShotRefusalNet::MagazineEmpty,
        },
        ActReply::ReloadRefused {
            reason: ReloadRefusalNet::AlreadyFull,
        },
        ActReply::MoveRefused {
            reason: MoveRejectionNet::Unreachable,
        },
        ActReply::StanceRefused {
            reason: StanceRefusalNet::AlreadyHeld,
        },
        ActReply::FacingRefused {
            reason: FacingRefusalNet::AlreadyFacing,
        },
    ];
    for refusal in refusals {
        assert_ne!(
            refusal, A_WINDOW,
            "Accepted means the intent reached the sim, so a typed refusal must never decode as \
             one",
        );
    }
}

#[test]
fn click_reply_round_trips_every_decision() {
    let decisions = [
        ClickDecisionNet::Fire,
        ClickDecisionNet::Select,
        ClickDecisionNet::SetMoveTarget,
        ClickDecisionNet::Move,
        ClickDecisionNet::NoOp,
        ClickDecisionNet::Clear,
    ];
    for decision in decisions {
        match decision {
            ClickDecisionNet::Fire
            | ClickDecisionNet::Select
            | ClickDecisionNet::SetMoveTarget
            | ClickDecisionNet::Move
            | ClickDecisionNet::NoOp
            | ClickDecisionNet::Clear => {}
        }
        assert_ron_round_trip(&decision);
        assert_ron_round_trip(&ClickReply {
            decision,
            act: None,
        });
    }
    assert_ron_round_trip(&ClickReply {
        decision: ClickDecisionNet::Move,
        act:      Some(ActReply::MoveRefused {
            reason: MoveRejectionNet::Suppressed,
        }),
    });
}

#[test]
fn select_reply_round_trips_every_variant() {
    let cases = [
        SelectReply::Selected {
            shooter: Some(GangerToken::new(42)),
        },
        SelectReply::Selected { shooter: None },
        SelectReply::Refused {
            reason: ActRefusalNet::UnknownToken,
        },
    ];
    for case in cases {
        match case {
            SelectReply::Selected { .. } | SelectReply::Refused { .. } => {}
        }
        assert_ron_round_trip(&case);
    }
}

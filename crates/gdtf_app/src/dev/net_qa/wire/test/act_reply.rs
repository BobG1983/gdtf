use crate::dev::net_qa::wire::{
    act::{ActCompleteNet, ActRefusalNet, ActReply, ActSeqNet, SelectReply},
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
fn act_reply_round_trips_every_variant() {
    let cases = [
        ActReply::Accepted {
            from_seq: ActSeqNet::new(7),
            to_seq:   ActSeqNet::new(11),
            complete: ActCompleteNet::new(false),
        },
        ActReply::Refused {
            reason: ActRefusalNet::NoShooter,
        },
    ];
    for case in cases {
        match case {
            ActReply::Accepted { .. } | ActReply::Refused { .. } => {}
        }
        assert_ron_round_trip(&case);
    }
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

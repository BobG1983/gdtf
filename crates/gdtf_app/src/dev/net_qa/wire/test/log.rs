use gdtf_battle_sim::act_log::ActProvenance;

use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    act::ActSeqNet,
    deed::ActDeedKindNet,
    log::{ActProvenanceNet, LogDroppedCount, LogEntryNet, LogReadCap},
    token::GangerToken,
};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("an act-log value serializes to compact RON");
    };
    text
}

#[test]
fn act_provenance_round_trips_every_variant() {
    for provenance in [
        ActProvenanceNet::Commanded,
        ActProvenanceNet::AiTurn,
        ActProvenanceNet::Reaction {
            interrupted: GangerToken::new(21),
        },
        ActProvenanceNet::Clock,
    ] {
        match provenance {
            ActProvenanceNet::Commanded
            | ActProvenanceNet::AiTurn
            | ActProvenanceNet::Reaction { .. }
            | ActProvenanceNet::Clock => {}
        }
        assert_ron_round_trip(&provenance);
    }
}

#[test]
fn log_scalars_round_trip() {
    assert_ron_round_trip(&ActSeqNet::new(0));
    assert_ron_round_trip(&ActSeqNet::new(4_294_967_296));
    assert_ron_round_trip(&LogReadCap::new(64));
    assert_ron_round_trip(&LogDroppedCount::new(7));
}

#[test]
fn log_values_serialize_transparently() {
    assert_eq!(encoded(&ActSeqNet::new(12)), "12");
    assert_eq!(encoded(&LogReadCap::new(64)), "64");
    assert_eq!(encoded(&LogDroppedCount::new(7)), "7");
    assert_eq!(
        encoded(&ActProvenanceNet::Reaction {
            interrupted: GangerToken::new(21),
        }),
        "Reaction(interrupted:21)",
    );
}

#[test]
fn a_log_entry_round_trips() {
    let entry: LogEntryNet = LogEntryNet {
        seq:        ActSeqNet::new(19),
        actor:      GangerToken::new(4_294_967_296),
        provenance: ActProvenanceNet::Reaction {
            interrupted: GangerToken::new(8),
        },
        kind:       ActDeedKindNet::RoundResolved,
    };
    assert_ron_round_trip(&entry);
}

#[test]
fn a_sim_provenance_mirrors_onto_the_wire() {
    assert_eq!(
        ActProvenanceNet::from_sim(ActProvenance::AiTurn),
        ActProvenanceNet::AiTurn,
        "the mirror must carry the sim's own provenance",
    );
    assert_eq!(
        ActProvenanceNet::from_sim(ActProvenance::Clock),
        ActProvenanceNet::Clock,
        "a clock-driven act mirrors as clock-driven",
    );
}

#[test]
fn a_log_entry_refuses_an_unknown_field() {
    let hostile = "(seq:1,actor:2,provenance:Clock,kind:Bled,extra:3)";
    assert!(
        ron::de::from_str::<LogEntryNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}

#[test]
fn act_sequence_numbers_order() {
    assert!(
        ActSeqNet::new(3) < ActSeqNet::new(4),
        "an earlier act sorts before a later one",
    );
}

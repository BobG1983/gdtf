use bevy::prelude::Entity;

use crate::{
    act_log::{ActDeed, ActEntry, ActLog, ActLogCapacity, ActProvenance, ActSeq, RecordedAct},
    ganger::Faction,
};

fn beat(gang: u8) -> RecordedAct {
    RecordedAct::new(
        Entity::PLACEHOLDER,
        ActProvenance::Clock,
        ActDeed::TurnBegan {
            now_active: Faction::new(gang),
        },
    )
}

#[test]
fn appending_assigns_consecutive_sequence_numbers_from_zero() {
    let mut log = ActLog::default();
    assert_eq!(
        log.head(),
        ActSeq::START,
        "a fresh log starts at sequence 0"
    );

    let first = log.append(beat(0));
    let second = log.append(beat(1));

    assert_eq!(first, ActSeq::new(0));
    assert_eq!(second, ActSeq::new(1));
    assert_eq!(
        log.head(),
        ActSeq::new(2),
        "head is one PAST the last appended entry — what a caught-up cursor equals",
    );
}

#[test]
fn since_is_non_destructive_so_two_cursors_coexist() {
    let mut log = ActLog::default();
    for gang in 0..4 {
        log.append(beat(gang));
    }

    let first_pass: Vec<ActSeq> = log.since(ActSeq::START).map(ActEntry::seq).collect();
    let second_pass: Vec<ActSeq> = log.since(ActSeq::START).map(ActEntry::seq).collect();

    assert_eq!(
        first_pass,
        vec![
            ActSeq::new(0),
            ActSeq::new(1),
            ActSeq::new(2),
            ActSeq::new(3)
        ],
    );
    assert_eq!(
        first_pass, second_pass,
        "a second reader must see exactly what the first saw — reading never consumes",
    );

    let tail: Vec<ActSeq> = log.since(ActSeq::new(2)).map(ActEntry::seq).collect();
    assert_eq!(
        tail,
        vec![ActSeq::new(2), ActSeq::new(3)],
        "a cursor mid-buffer resumes at its own position, inclusive",
    );
}

#[test]
fn capacity_drops_oldest_and_counts() {
    let mut log = ActLog::new(ActLogCapacity::new(3));
    for gang in 0..5 {
        log.append(beat(gang));
    }

    assert_eq!(log.len(), 3, "the ring retains exactly its capacity");
    assert_eq!(
        *log.dropped(),
        2,
        "the two evicted entries are counted, not silently lost",
    );
    assert_eq!(
        log.oldest_seq(),
        ActSeq::new(2),
        "the oldest RETAINED entry is what a reader compares its cursor against to detect \
         that it fell off the window",
    );
    assert_eq!(
        log.head(),
        ActSeq::new(5),
        "eviction does not rewind the sequence counter — every append still advances it",
    );
    let retained: Vec<ActSeq> = log.since(ActSeq::START).map(ActEntry::seq).collect();
    assert_eq!(
        retained,
        vec![ActSeq::new(2), ActSeq::new(3), ActSeq::new(4)],
        "a cursor that fell off the window still reads everything retained, oldest first",
    );
}

#[test]
fn an_empty_log_reports_head_as_its_oldest_sequence() {
    let log = ActLog::default();
    assert!(log.is_empty());
    assert_eq!(log.oldest_seq(), log.head());
    assert_eq!(log.since(ActSeq::START).count(), 0);
}

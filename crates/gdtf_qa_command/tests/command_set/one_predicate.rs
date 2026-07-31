//! `admit()` refuses EXACTLY what `catalogue()` marks unavailable.
//!
//! Advertised and admitted are computed from the same call — the command's own
//! availability predicate — so this walks the catalogue a host would publish and, for every
//! row, asks `admit` the same question. Across two distinct facts values, the two answers
//! agree row for row.

use gdtf_qa_command::{
    catalogue::catalogue,
    dispatch::{Admission, admit},
    test_support::{
        FAKE_COMMANDS, FAKE_COMMANDS_GROWN, FakeFacts, fake_facts_loaded, fake_facts_unloaded,
        fake_host_name,
    },
};
use gdtf_qa_protocol::command::CommandAvailability;

use crate::support::plain;

/// Every catalogue row's availability is exactly what `admit` decides for that name.
fn advertised_and_admitted_agree(facts: FakeFacts) {
    let published = catalogue(fake_host_name(), FAKE_COMMANDS, &facts);
    assert_eq!(
        published.entries.len(),
        FAKE_COMMANDS.len(),
        "the catalogue publishes one row per command"
    );

    for entry in &published.entries {
        let decision = admit(FAKE_COMMANDS, &entry.command, &plain(), &facts);
        match (&entry.availability, decision) {
            (CommandAvailability::Available, Admission::Admit(_)) => {}
            (CommandAvailability::Unavailable { code, note }, Admission::Unavailable(refusal)) => {
                assert_eq!(
                    refusal.to_availability(),
                    CommandAvailability::Unavailable {
                        code: *code,
                        note: note.clone(),
                    },
                    "{} advertised one refusal and admitted another",
                    entry.command.as_str()
                );
            }
            (availability, decision) => unreachable!(
                "{} advertised {availability:?} but admission said {decision:?}",
                entry.command.as_str()
            ),
        }
    }
}

/// With nothing loaded, advertised and admitted agree — including on the refusals.
#[test]
fn they_agree_with_nothing_loaded() {
    advertised_and_admitted_agree(fake_facts_unloaded());
}

/// With the model loaded, advertised and admitted agree again.
#[test]
fn they_agree_with_the_model_loaded() {
    advertised_and_admitted_agree(fake_facts_loaded());
}

/// The two facts values really do produce DIFFERENT catalogues.
///
/// Without this the two agreement tests above would be satisfied by a predicate that always
/// says `Available` — they would pass while proving nothing.
#[test]
fn the_two_facts_values_produce_different_availability() {
    let unloaded = catalogue(
        fake_host_name(),
        FAKE_COMMANDS_GROWN,
        &fake_facts_unloaded(),
    );
    let loaded = catalogue(fake_host_name(), FAKE_COMMANDS_GROWN, &fake_facts_loaded());

    let refused = |published: &gdtf_qa_protocol::command::CommandCatalogue| {
        published
            .entries
            .iter()
            .filter(|entry| entry.availability != CommandAvailability::Available)
            .count()
    };

    assert_eq!(
        refused(&unloaded),
        2,
        "fake.cell and fake.echo are both refused with nothing loaded"
    );
    assert_eq!(refused(&loaded), 0, "everything is available once loaded");
}

/// The refusal a client is shown is the command's own words, not a generic one.
#[test]
fn the_advertised_refusal_carries_the_command_s_own_note() {
    let published = catalogue(fake_host_name(), FAKE_COMMANDS, &fake_facts_unloaded());
    let Some(entry) = published
        .entries
        .iter()
        .find(|entry| entry.command.as_str() == "fake.cell")
    else {
        unreachable!("fake.cell is in the fake set");
    };
    let CommandAvailability::Unavailable { note, .. } = &entry.availability else {
        unreachable!("fake.cell is unavailable with nothing loaded");
    };
    assert_eq!(note.as_str(), "the fake host has loaded no cell model");
}

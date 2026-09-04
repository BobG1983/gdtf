use cobalt_mcp_command::{
    catalogue::catalogue,
    dispatch::{Admission, admit},
    test_support::{
        FAKE_COMMANDS, FAKE_COMMANDS_GROWN, FakeFacts, fake_facts_loaded, fake_facts_unloaded,
        fake_host_name,
    },
};
use cobalt_mcp_protocol::command::CommandAvailability;

fn advertised_and_admitted_agree(facts: FakeFacts) {
    let published = catalogue(fake_host_name(), FAKE_COMMANDS, &facts);
    assert_eq!(
        published.entries.len(),
        FAKE_COMMANDS.len(),
        "the catalogue publishes one row per command"
    );

    for entry in &published.entries {
        let decision = admit(FAKE_COMMANDS, &entry.command, &facts);
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

#[test]
fn they_agree_with_nothing_loaded() {
    advertised_and_admitted_agree(fake_facts_unloaded());
}

#[test]
fn they_agree_with_the_model_loaded() {
    advertised_and_admitted_agree(fake_facts_loaded());
}

#[test]
fn the_two_facts_values_produce_different_availability() {
    let unloaded = catalogue(
        fake_host_name(),
        FAKE_COMMANDS_GROWN,
        &fake_facts_unloaded(),
    );
    let loaded = catalogue(fake_host_name(), FAKE_COMMANDS_GROWN, &fake_facts_loaded());

    let refused = |published: &cobalt_mcp_protocol::command::CommandCatalogue| {
        published
            .entries
            .iter()
            .filter(|entry| entry.availability != CommandAvailability::Available)
            .count()
    };

    assert_eq!(
        refused(&unloaded),
        2,
        "fake.point and fake.echo are both refused with nothing loaded"
    );
    assert_eq!(refused(&loaded), 0, "everything is available once loaded");
}

#[test]
fn the_advertised_refusal_carries_the_command_s_own_note() {
    let published = catalogue(fake_host_name(), FAKE_COMMANDS, &fake_facts_unloaded());
    let Some(entry) = published
        .entries
        .iter()
        .find(|entry| entry.command.as_str() == "fake.point")
    else {
        unreachable!("fake.point is in the fake set");
    };
    let CommandAvailability::Unavailable { note, .. } = &entry.availability else {
        unreachable!("fake.point is unavailable with nothing loaded");
    };
    assert_eq!(note.as_str(), "the fake host has loaded no point model");
}

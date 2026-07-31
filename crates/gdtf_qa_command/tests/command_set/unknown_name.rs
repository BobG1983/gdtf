//! An unknown name is answered `Unknown`, carrying EVERY known name.

use gdtf_qa_command::test_support::{FAKE_COMMANDS, fake_app, fake_facts_loaded, run_fake_command};
use gdtf_qa_protocol::command::{CommandName, CommandOutcome};

use crate::support::{args, outcome, plain};

/// A near-miss name is refused with the whole known set, so a typo self-corrects in one
/// round trip.
#[test]
fn an_unknown_name_is_answered_with_every_known_name() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &CommandName::from_static("fake.phasee"),
        &args("{}"),
        &plain(),
    );

    // Answered at route time — no frame needs to run at all.
    let answered = outcome(&channel);
    let CommandOutcome::Unknown { known } = answered else {
        unreachable!("an unknown name must answer Unknown, got {answered:?}");
    };

    let names: Vec<&str> = known.iter().map(CommandName::as_str).collect();
    assert_eq!(names, vec!["fake.phase", "fake.cell", "fake.settle"]);
}

/// The known list grows with the set — it is read from the same slice, not hand-kept.
#[test]
fn the_known_list_is_read_from_the_host_s_own_slice() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        gdtf_qa_command::test_support::FAKE_COMMANDS_GROWN,
        &CommandName::from_static("fake.nothing"),
        &args("{}"),
        &plain(),
    );

    let answered = outcome(&channel);
    let CommandOutcome::Unknown { known } = answered else {
        unreachable!("an unknown name must answer Unknown, got {answered:?}");
    };
    assert_eq!(known.len(), 4, "the grown slice reports four known names");
}

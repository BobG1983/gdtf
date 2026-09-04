use cobalt_mcp_host::test_support::{FAKE_COMMANDS, fake_app, fake_facts_loaded, run_fake_command};
use cobalt_mcp_protocol::command::{CommandName, CommandOutcome};

use crate::support::{args, outcome, plain};

#[test]
fn an_unknown_name_is_answered_with_every_known_name() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &CommandName::from_static("fake.phasee"),
        &args("()"),
        &plain(),
    );

    let answered = outcome(&channel);
    let CommandOutcome::Unknown { known } = answered else {
        unreachable!("an unknown name must answer Unknown, got {answered:?}");
    };

    let names: Vec<&str> = known.iter().map(CommandName::as_str).collect();
    assert_eq!(names, vec!["fake.phase", "fake.point", "fake.settle"]);
}

#[test]
fn the_known_list_is_read_from_the_host_s_own_slice() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        cobalt_mcp_host::test_support::FAKE_COMMANDS_GROWN,
        &CommandName::from_static("fake.nothing"),
        &args("()"),
        &plain(),
    );

    let answered = outcome(&channel);
    let CommandOutcome::Unknown { known } = answered else {
        unreachable!("an unknown name must answer Unknown, got {answered:?}");
    };
    assert_eq!(known.len(), 4, "the grown slice reports four known names");
}

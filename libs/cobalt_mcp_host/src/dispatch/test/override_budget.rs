use core::time::Duration;

use bevy::prelude::*;
use cobalt_mcp_protocol::{
    command::{CommandArgsRon, RunOptions},
    message::{McpResponse, McpSessionError},
};

use crate::{
    command::McpCommand,
    dispatch::{
        DeferredBudget, DeferredBudgetOverride, DeferredReplies, register_command_set,
        register_riders,
    },
    test_support::{FAKE_COMMANDS, FakeSettle, fake_facts_loaded, run_fake_command},
};

fn app_registered_with(budget: Duration) -> App {
    let mut app = App::new();
    app.insert_resource(fake_facts_loaded());
    app.insert_resource(DeferredBudgetOverride::new(DeferredBudget::new(budget)));
    register_riders(&mut app);
    register_command_set(&mut app, FAKE_COMMANDS);
    app
}

fn park_one_settle(app: &mut App) -> std::sync::mpsc::Receiver<McpResponse> {
    run_fake_command(
        app,
        FAKE_COMMANDS,
        &FakeSettle::NAME,
        &CommandArgsRon::new("()".to_owned()),
        &RunOptions::default(),
    )
}

#[test]
fn an_override_budget_of_zero_expires_the_parked_reply_on_its_first_frame() {
    let mut app = app_registered_with(Duration::ZERO);
    assert_eq!(
        app.world()
            .resource::<DeferredReplies<FakeSettle>>()
            .budget(),
        DeferredBudget::new(Duration::ZERO),
        "the override must win over the command's own declared budget",
    );
    let channel = park_one_settle(&mut app);

    app.update();

    assert_eq!(
        channel.try_recv().ok(),
        Some(McpResponse::Error(McpSessionError::Timeout)),
        "a zero override budget expires the parked reply on the sweep of its own frame",
    );
}

#[test]
fn an_override_budget_of_the_longest_duration_leaves_the_reply_parked() {
    let mut app = app_registered_with(Duration::MAX);
    assert_eq!(
        app.world()
            .resource::<DeferredReplies<FakeSettle>>()
            .budget(),
        DeferredBudget::new(Duration::MAX),
        "the override must win over the command's own declared budget",
    );
    let channel = park_one_settle(&mut app);

    app.update();

    assert!(
        channel.try_recv().is_err(),
        "the longest override budget must leave the reply parked, not swept",
    );
    assert_eq!(
        app.world().resource::<DeferredReplies<FakeSettle>>().len(),
        1,
        "the parked entry is still waiting to settle",
    );
}

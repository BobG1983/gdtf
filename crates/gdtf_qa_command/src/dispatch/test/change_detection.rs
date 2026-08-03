use bevy::prelude::*;
use gdtf_qa_protocol::command::{CommandArgsJson, RunOptions};

use crate::{
    command::QaCommand,
    dispatch::{CommandInbox, DeferredReplies},
    test_support::{
        FAKE_COMMANDS, FakePhase, FakeSettle, fake_app, fake_facts_loaded, run_fake_command,
    },
};

fn inbox_was_dirtied(app: &App) -> bool {
    let Some(inbox) = app.world().get_resource_ref::<CommandInbox>() else {
        unreachable!("registering a command initialises the inbox");
    };
    inbox.is_changed()
}

fn deferred_was_dirtied(app: &App) -> bool {
    let Some(deferred) = app
        .world()
        .get_resource_ref::<DeferredReplies<FakeSettle>>()
    else {
        unreachable!("registering a command initialises its deferral parking");
    };
    deferred.is_changed()
}

fn idle_frame(app: &mut App) {
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Last);
}

#[test]
fn an_idle_frame_dirties_neither_shared_resource() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    app.update();

    idle_frame(&mut app);

    assert!(
        !inbox_was_dirtied(&app),
        "an empty inbox read through ResMut must not dirty change detection"
    );
    assert!(
        !deferred_was_dirtied(&app),
        "empty deferral parking read through ResMut must not dirty change detection"
    );
}

#[test]
fn a_frame_with_work_dirties_both_shared_resources() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    app.update();

    for name in [FakePhase::NAME, FakeSettle::NAME] {
        let _answer = run_fake_command(
            &mut app,
            FAKE_COMMANDS,
            &name,
            &CommandArgsJson::new("{}".to_owned()),
            &RunOptions::default(),
        );
    }
    idle_frame(&mut app);

    assert!(
        inbox_was_dirtied(&app),
        "claiming a call takes the inbox by &mut"
    );
    assert!(
        deferred_was_dirtied(&app),
        "parking a reply takes the deferral parking by &mut"
    );
}

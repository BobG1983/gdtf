//! An IDLE frame dirties neither shared resource.
//!
//! [`CommandInbox::is_empty`](crate::dispatch::CommandInbox::is_empty) and
//! [`DeferredReplies::is_empty`](crate::dispatch::DeferredReplies::is_empty) take `&self`
//! precisely so the early-outs in `claim_calls` and `sweep_deferred` can read them through
//! a `ResMut` without touching change detection. Widen either signature to `&mut self` and
//! the code still compiles at both call sites — a host with fifty commands would then dirty
//! both resources fifty times on every idle frame, and nothing but this file would notice.
//!
//! The frames are driven with `run_schedule` rather than `App::update`, because `update`
//! ends with `clear_trackers`, which advances `last_change_tick` past everything the frame
//! did and would make the assertion pass no matter what.

use bevy::prelude::*;
use gdtf_qa_protocol::command::{CommandArgsJson, RunOptions};

use crate::{
    command::QaCommand,
    dispatch::{CommandInbox, DeferredReplies},
    test_support::{
        FAKE_COMMANDS, FakePhase, FakeSettle, fake_app, fake_facts_loaded, run_fake_command,
    },
};

/// Whether the shared inbox was dirtied since the last tracker clear.
fn inbox_was_dirtied(app: &App) -> bool {
    let Some(inbox) = app.world().get_resource_ref::<CommandInbox>() else {
        unreachable!("registering a command initialises the inbox");
    };
    inbox.is_changed()
}

/// Whether `FakeSettle`'s deferral parking was dirtied since the last tracker clear.
fn deferred_was_dirtied(app: &App) -> bool {
    let Some(deferred) = app
        .world()
        .get_resource_ref::<DeferredReplies<FakeSettle>>()
    else {
        unreachable!("registering a command initialises its deferral parking");
    };
    deferred.is_changed()
}

/// Run one frame's two schedules WITHOUT clearing the change trackers afterwards.
fn idle_frame(app: &mut App) {
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Last);
}

/// A frame with nothing to do leaves both shared resources unchanged.
#[test]
fn an_idle_frame_dirties_neither_shared_resource() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    // One full update first, so `clear_trackers` puts the resources' creation behind us.
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

/// A frame that ACTUALLY has work dirties both — so the test above is observing the early
/// out and not a change-detection reading that never says yes.
#[test]
fn a_frame_with_work_dirties_both_shared_resources() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    app.update();

    // One call for each: `fake.phase` passes through the inbox, `fake.settle` parks.
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

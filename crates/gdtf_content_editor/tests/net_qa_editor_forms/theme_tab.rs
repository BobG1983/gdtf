//! The Theme tab, which owns no field arm and no list of its own, driven with both write
//! commands: a field or list belonging to another form, and a draft that leaves the world.

use bevy::prelude::*;
use gdtf_content_editor::{EditorMode, ThemeDraft};
use gdtf_qa_command::dispatch::{CommandInbox, QaCommandSystems};
use gdtf_qa_protocol::message::QaResponse;

use crate::{
    outcome::unavailable_code,
    refusal::refusal_note,
    setup::{form_tab_app_and_client, gang_draft, try_list_op, try_set_field},
    socket::Client,
    support::{TestError, TestResult},
};

// A field arm the Theme form does not own. Its route sends every arm to the same helper.
const A_GANG_FIELD: &str = "(field: Gang(Name(\"Ash Ferals\")))";

// A list the Theme form does not own.
const A_GANG_LIST: &str = "(list: GangMembers, op: Add)";

// The route's foreign-arm refusal.
fn assert_the_foreign_arm_answered(reply: &QaResponse) -> TestResult {
    assert_eq!(
        unavailable_code(reply)?,
        "WrongState",
        "an arm belonging to another form is refused rather than written",
    );
    let note = refusal_note(reply)?;
    assert!(
        note.contains("belongs to another form"),
        "the foreign-arm wording is what separates this refusal from the one a missing draft \
         answers: `{note}`",
    );
    assert!(
        note.contains("Theme"),
        "the note names the tab that is open, so a client can see which form it reached: \
         `{note}`",
    );
    Ok(())
}

// The route's own missing-draft refusal.
fn assert_the_draft_gone_arm_answered(reply: &QaResponse) -> TestResult {
    assert_eq!(
        unavailable_code(reply)?,
        "WrongState",
        "a draft that left the world is host state, so the route names the code the catalogue \
         names, not MissingModel",
    );
    let note = refusal_note(reply)?;
    assert!(
        note.contains("Theme"),
        "the route's own refusal names the open mode, so a note without it came from the \
         availability check instead: `{note}`",
    );
    assert!(
        note.contains("draft resource is not in the world"),
        "the missing-draft wording is what separates this refusal from the foreign-arm one: \
         `{note}`",
    );
    Ok(())
}

// Take the Theme draft out of the world in the frame that already admitted the call.
fn drop_the_theme_draft_after_admission(world: &mut World) {
    let admitted = world
        .get_resource::<CommandInbox>()
        .is_some_and(|inbox| !inbox.is_empty());
    if admitted {
        world.remove_resource::<ThemeDraft>();
    }
}

// A Theme app whose draft leaves the world between the router and the handler.
fn theme_app_losing_its_draft_mid_frame() -> Result<(App, Client), TestError> {
    let (mut app, client) = form_tab_app_and_client(EditorMode::Theme)?;
    app.add_systems(
        Update,
        drop_the_theme_draft_after_admission
            .after(QaCommandSystems::Route)
            .before(QaCommandSystems::Claim),
    );
    Ok((app, client))
}

#[test]
fn a_gang_field_on_the_theme_tab_is_refused_as_a_foreign_arm() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Theme)?;
    let before = gang_draft(&app)?;

    let reply = try_set_field(&mut app, &mut client, A_GANG_FIELD)?;
    assert_the_foreign_arm_answered(&reply)?;
    assert_eq!(
        gang_draft(&app)?,
        before,
        "a Theme writer that fell through to the Gang draft would write it here",
    );
    Ok(())
}

#[test]
fn a_set_field_on_the_theme_tab_whose_draft_goes_mid_frame_is_refused_by_the_route() -> TestResult {
    let (mut app, mut client) = theme_app_losing_its_draft_mid_frame()?;

    let reply = try_set_field(&mut app, &mut client, A_GANG_FIELD)?;
    assert_the_draft_gone_arm_answered(&reply)?;
    assert!(
        app.world().get_resource::<ThemeDraft>().is_none(),
        "the case only proves the route's own arm if the draft really left the world",
    );
    Ok(())
}

#[test]
fn a_gang_list_on_the_theme_tab_is_refused_as_a_foreign_arm() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Theme)?;
    let before = gang_draft(&app)?;

    let reply = try_list_op(&mut app, &mut client, A_GANG_LIST)?;
    assert_the_foreign_arm_answered(&reply)?;
    assert_eq!(
        gang_draft(&app)?,
        before,
        "the Gang route answers this list with a new member, so a Theme tab falling through to \
         it would show here",
    );
    Ok(())
}

#[test]
fn a_list_op_on_the_theme_tab_whose_draft_goes_mid_frame_is_refused_by_the_route() -> TestResult {
    let (mut app, mut client) = theme_app_losing_its_draft_mid_frame()?;

    let reply = try_list_op(&mut app, &mut client, A_GANG_LIST)?;
    assert_the_draft_gone_arm_answered(&reply)?;
    assert!(
        app.world().get_resource::<ThemeDraft>().is_none(),
        "the case only proves the route's own arm if the draft really left the world",
    );
    Ok(())
}

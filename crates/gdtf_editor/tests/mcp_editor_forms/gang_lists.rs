//! The Gang form's member roster, driven over the listener with the Gang tab open.

use gdtf_editor::{EditorMode, GangDraft};

use crate::{
    bad_arguments::bad_arguments_detail,
    rows::{ListMemberRow, ListRow},
    setup::{form_tab_app_and_client, gang_draft, list_op, try_list_op},
    support::TestResult,
};

// The members the draft holds, as the reply reads them back.
fn names_of(draft: &GangDraft) -> Vec<ListMemberRow> {
    draft
        .members()
        .iter()
        .map(|member| ListMemberRow::GangMember(member.name.as_str().to_owned()))
        .collect()
}

#[test]
fn an_add_then_a_remove_reads_the_roster_back_each_time() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    assert!(
        gang_draft(&app)?.members().is_empty(),
        "a Gang tab reached over the listener holds the form's default draft",
    );

    let added = list_op(&mut app, &mut client, "(list: GangMembers, op: Add)")?;
    assert_eq!(added.list, ListRow::GangMembers);
    let held = gang_draft(&app)?;
    assert_eq!(held.members().len(), 1, "Add appends the default member");
    assert_eq!(
        added.members,
        names_of(&held),
        "the reply names the members the draft holds, in order",
    );

    let removed = list_op(&mut app, &mut client, "(list: GangMembers, op: Remove(0))")?;
    let held = gang_draft(&app)?;
    assert!(
        held.members().is_empty(),
        "the roster keeps no minimum, so the last member comes off",
    );
    assert!(
        removed.members.is_empty(),
        "the reply reads the empty roster back",
    );
    Ok(())
}

#[test]
fn two_adds_read_back_in_the_order_the_roster_holds_them() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    list_op(&mut app, &mut client, "(list: GangMembers, op: Add)")?;

    let second = list_op(&mut app, &mut client, "(list: GangMembers, op: Add)")?;

    let held = gang_draft(&app)?;
    assert_eq!(held.members().len(), 2);
    assert_eq!(second.members, names_of(&held));
    Ok(())
}

#[test]
fn a_remove_past_the_end_of_the_roster_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    list_op(&mut app, &mut client, "(list: GangMembers, op: Add)")?;
    let before = gang_draft(&app)?;

    let reply = try_list_op(&mut app, &mut client, "(list: GangMembers, op: Remove(1))")?;

    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains("past the end"),
        "the detail says the index named no row, got `{detail}`",
    );
    assert_eq!(
        gang_draft(&app)?,
        before,
        "the refused remove left the roster exactly as it was",
    );
    Ok(())
}

#[test]
fn a_reorder_of_the_member_roster_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    list_op(&mut app, &mut client, "(list: GangMembers, op: Add)")?;
    let before = gang_draft(&app)?;

    let reply = try_list_op(&mut app, &mut client, "(list: GangMembers, op: MoveUp(0))")?;

    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains("GangMembers"),
        "the detail names the list that draws no reorder, got `{detail}`",
    );
    assert_eq!(gang_draft(&app)?, before);
    Ok(())
}

// The Gang roster is the one list a client might mistake for a toggle list.
#[test]
fn a_toggle_of_the_member_roster_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    let before = gang_draft(&app)?;

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: GangMembers, op: Toggle(EntrySide(East)))",
    )?;

    let detail: String = bad_arguments_detail(&reply)?;
    assert!(detail.contains("GangMembers"), "got `{detail}`");
    assert_eq!(gang_draft(&app)?, before);
    Ok(())
}

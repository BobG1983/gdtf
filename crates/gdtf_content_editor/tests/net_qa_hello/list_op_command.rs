use bevy::app::App;
use gdtf_battle_sim::terrain::facing::TerrainFacing;
use gdtf_content_editor::TerrainDraft;

use crate::{
    client::{EDITOR_LIST_OP, EDITOR_SET_FIELD, EDITOR_SET_MODE},
    harness::editing_app_and_client,
    load_case::reply_answered_during_load,
    outcome::{ran_body, unavailable_code},
    rows::{FacingRow, ListOpReplyRow, ListRow},
    socket::{Client, run_editor},
    support::{TestError, TestResult},
};

const TOGGLE_EAST: &str = "(list: EntrySides, op: Toggle(East))";

fn entry_sides(app: &App) -> Result<Vec<TerrainFacing>, TestError> {
    let Some(draft) = app.world().get_resource::<TerrainDraft>() else {
        return Err(
            "the terrain draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.entry_sides().to_vec())
}

fn terrain_emplacement_case() -> Result<(App, Client), TestError> {
    let (mut app, mut client) = editing_app_and_client()?;
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Terrain)"))?;
    client.exchange(
        &mut app,
        &run_editor(EDITOR_SET_FIELD, "(field: Kind(Emplacement))"),
    )?;
    Ok((app, client))
}

#[test]
fn a_toggle_adds_the_side_then_takes_it_back_off_reply_and_world_agree() -> TestResult {
    let (mut app, mut client) = terrain_emplacement_case()?;
    assert!(
        entry_sides(&app)?.is_empty(),
        "a fresh Emplacement draft names no entry side, or the toggles below would prove nothing",
    );

    let added = client.exchange(&mut app, &run_editor(EDITOR_LIST_OP, TOGGLE_EAST))?;
    let body: ListOpReplyRow = ran_body(&added, EDITOR_LIST_OP)?;
    assert_eq!(
        body.list,
        ListRow::EntrySides,
        "the reply names the list it touched"
    );
    assert_eq!(
        body.members,
        vec![FacingRow::East],
        "the reply reads back the list the write left, holding only the toggled side",
    );
    assert_eq!(
        entry_sides(&app)?,
        vec![TerrainFacing::East],
        "the world's own Terrain draft holds the toggled side on the frame that answered",
    );

    let removed = client.exchange(&mut app, &run_editor(EDITOR_LIST_OP, TOGGLE_EAST))?;
    let body: ListOpReplyRow = ran_body(&removed, EDITOR_LIST_OP)?;
    assert!(
        body.members.is_empty(),
        "toggling the same side again takes it back off, got {:?}",
        body.members,
    );
    assert!(
        entry_sides(&app)?.is_empty(),
        "the world's own draft is empty again on the frame that answered",
    );
    Ok(())
}

#[test]
fn a_toggle_on_a_non_emplacement_kind_is_refused() -> TestResult {
    let (mut app, mut client) = terrain_emplacement_case()?;
    client.exchange(
        &mut app,
        &run_editor(EDITOR_SET_FIELD, "(field: Kind(Wall))"),
    )?;

    let reply = client.exchange(&mut app, &run_editor(EDITOR_LIST_OP, TOGGLE_EAST))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft commits entry sides only on an Emplacement kind, so a Wall draft is refused \
         rather than written through a setter that would silently do nothing",
    );
    assert!(
        entry_sides(&app)?.is_empty(),
        "the refused write left no side on the draft",
    );
    Ok(())
}

#[test]
fn a_toggle_is_refused_on_another_tab() -> TestResult {
    let (mut app, mut client) = terrain_emplacement_case()?;
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Armor)"))?;

    let reply = client.exchange(&mut app, &run_editor(EDITOR_LIST_OP, TOGGLE_EAST))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "every list this command offers belongs to the Terrain draft, so another open tab is \
         refused rather than written through",
    );
    assert!(
        entry_sides(&app)?.is_empty(),
        "the refused write left no side on the draft",
    );
    Ok(())
}

#[test]
fn a_toggle_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        run_editor(EDITOR_LIST_OP, TOGGLE_EAST),
        "the editor.list_op run",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft is state-scoped to Editing, so a Load-pass call is refused rather than \
         creating one",
    );
    Ok(())
}

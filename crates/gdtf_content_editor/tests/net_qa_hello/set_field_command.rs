use bevy::app::App;
use gdtf_content_editor::{TerrainDraft, TerrainKindChoice};

use crate::{
    client::{EDITOR_SET_FIELD, EDITOR_SET_MODE},
    harness::editing_app_and_client,
    load_case::reply_answered_during_load,
    outcome::{ran_body, unavailable_code},
    rows::{FieldRow, SetFieldReplyRow, TerrainKindRow},
    socket::run_editor,
    support::{TestError, TestResult},
};

fn terrain_draft(app: &App) -> Result<TerrainDraft, TestError> {
    let Some(draft) = app.world().get_resource::<TerrainDraft>() else {
        return Err(
            "the terrain draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

#[test]
fn set_field_writes_the_terrain_kind_the_reply_and_the_world_agree() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Terrain)"))?;
    assert_ne!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::Emplacement,
        "the case must ask for a kind the draft is not already on, or it would pass against a \
         handler that writes nothing",
    );

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_SET_FIELD, "(field: Kind(Emplacement))"),
    )?;
    let body: SetFieldReplyRow = ran_body(&reply, EDITOR_SET_FIELD)?;

    assert_eq!(
        body.field,
        FieldRow::Kind(TerrainKindRow::Emplacement),
        "the reply names the field that was written, read back off the draft",
    );
    assert_eq!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::Emplacement,
        "the world's own Terrain draft is on the requested kind on the frame that answered — a \
         reply-only assertion would pass against a handler that writes nothing",
    );
    Ok(())
}

#[test]
fn set_field_is_refused_on_another_tab() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Armor)"))?;

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_SET_FIELD, "(field: Kind(Emplacement))"),
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "every field this command offers belongs to the Terrain draft, so another open tab is \
         refused rather than written through",
    );
    assert_eq!(
        terrain_draft(&app)?.kind(),
        TerrainKindChoice::default(),
        "a refused write leaves the Terrain draft exactly as it was",
    );
    Ok(())
}

#[test]
fn set_field_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        run_editor(EDITOR_SET_FIELD, "(field: Kind(Emplacement))"),
        "the editor.set_field run",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft is state-scoped to Editing, so a Load-pass call is refused rather than \
         creating one",
    );
    Ok(())
}

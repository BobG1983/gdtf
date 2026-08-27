use bevy::app::App;
use gdtf_battle_sim::{
    armor::ArmorSpec,
    injuries::InjuryDef,
    terrain::def::{TerrainDef, TerrainUuid},
};
use gdtf_content_editor::{
    ArmorDraft, EditorMode, InjuryDraft, TerrainDraft, draft_to_def, draft_to_spec,
    draft_to_terrain_def,
};
use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};
use serde::de::DeserializeOwned;

use crate::{
    draft_reply::{DraftOutcomeRow, DraftReplyRow},
    harness::editing_app_and_client,
    load_case::reply_answered_during_load,
    mirror::ModeRow,
    names::{EDITOR_DRAFT, EDITOR_SET_FIELD, EDITOR_SET_MODE},
    outcome::{ran_body, unavailable_code},
    save_fault::SaveFaultRow,
    socket::{Client, run_editor},
    support::{TestError, TestResult},
};

// The refusal note a client reads to learn what to do instead.
fn unavailable_note(reply: &QaResponse) -> Result<String, TestError> {
    let QaResponse::Outcome(CommandOutcome::Unavailable { note, .. }) = reply else {
        return Err(format!("expected an Unavailable outcome, got {reply:?}").into());
    };
    Ok(note.as_str().to_owned())
}

fn live_mode(app: &App) -> Result<EditorMode, TestError> {
    let Some(mode) = app.world().get_resource::<EditorMode>() else {
        return Err("the mode tab is a resource the editor creates on entering Editing".into());
    };
    Ok(*mode)
}

fn draft_of<T: bevy::prelude::Resource + Clone>(app: &App, what: &str) -> Result<T, TestError> {
    let Some(draft) = app.world().get_resource::<T>() else {
        return Err(format!(
            "the {what} draft is a resource the editor creates on entering Editing"
        )
        .into());
    };
    Ok(draft.clone())
}

fn open_tab(app: &mut App, client: &mut Client, mode: &str) -> Result<(), TestError> {
    client.exchange(
        app,
        &run_editor(EDITOR_SET_MODE, &format!("(mode: {mode})")),
    )?;
    Ok(())
}

fn read_draft(app: &mut App, client: &mut Client) -> Result<DraftReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_DRAFT, "()"))?;
    ran_body(&reply, EDITOR_DRAFT)
}

// Parse the projected text back as the type that mode's writer writes, or say what came back.
fn parsed_as<T: DeserializeOwned>(body: &DraftReplyRow, what: &str) -> Result<T, TestError> {
    let DraftOutcomeRow::Ron(text) = &body.outcome else {
        return Err(format!("expected projected RON for {what}, got {body:?}").into());
    };
    Ok(ron::de::from_str::<T>(text)?)
}

// The tab was reached over the socket, so the reply's mode is the world's own.
fn assert_mode_is(app: &App, body: &DraftReplyRow, expected: ModeRow) -> Result<(), TestError> {
    assert_eq!(
        body.mode, expected,
        "the reply names the tab it read, so a client never has to guess which draft it got",
    );
    assert_eq!(
        format!("{:?}", body.mode),
        format!("{:?}", live_mode(app)?),
        "the reply's mode is the world's own EditorMode on the frame that answered",
    );
    Ok(())
}

#[test]
fn the_default_prefab_tab_is_refused_and_the_note_names_editor_map() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    assert_eq!(
        live_mode(&app)?,
        EditorMode::Prefab,
        "the editor opens on Prefab, so this case reads the tab a client lands on",
    );

    let reply = client.exchange(&mut app, &run_editor(EDITOR_DRAFT, "()"))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "Prefab holds no draft, so the read is refused for the open tab rather than answering \
         an empty projection",
    );
    let note = unavailable_note(&reply)?;
    assert!(
        note.contains("editor.map"),
        "the refusal points at the command that does read Prefab's state, so one round trip is \
         enough to fix the call: {note:?}",
    );
    Ok(())
}

#[test]
fn the_terrain_tab_projects_what_its_own_conversion_builds() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, "Terrain")?;

    let body = read_draft(&mut app, &mut client)?;
    assert_mode_is(&app, &body, ModeRow::Terrain)?;
    let projected: TerrainDef = parsed_as(&body, "the Terrain draft")?;
    let draft: TerrainDraft = draft_of(&app, "terrain")?;
    let Ok(expected) = draft_to_terrain_def(&draft, draft.uuid().unwrap_or_else(TerrainUuid::nil))
    else {
        return Err("the default Terrain draft is a Wall, which converts".into());
    };
    assert_eq!(
        projected, expected,
        "the text is what the form's own conversion builds, so a hand-rolled projection would \
         read back with a different shape",
    );
    assert_eq!(
        draft.uuid(),
        None,
        "the read takes the preview pane's key route, not the save path's `ensure_uuid`. A \
         read that assigned a uuid would leave one on the draft",
    );
    Ok(())
}

#[test]
fn the_armor_tab_projects_the_spec_alone_and_never_its_name_pair() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, "Armor")?;

    let body = read_draft(&mut app, &mut client)?;
    assert_mode_is(&app, &body, ModeRow::Armor)?;
    let projected: ArmorSpec = parsed_as(&body, "the Armor draft")?;
    let draft: ArmorDraft = draft_of(&app, "armor")?;
    assert_eq!(
        projected,
        draft_to_spec(&draft).1,
        "the value on the wire is the second element of the conversion's pair, which is what \
         the writer hands the RON file. Serializing the pair would not parse as an ArmorSpec",
    );
    Ok(())
}

#[test]
fn the_injury_tab_projects_the_injury_def_alone() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, "Injury")?;

    let body = read_draft(&mut app, &mut client)?;
    assert_mode_is(&app, &body, ModeRow::Injury)?;
    let projected: InjuryDef = parsed_as(&body, "the Injury draft")?;
    let draft: InjuryDraft = draft_of(&app, "injury")?;
    assert_eq!(
        projected,
        draft_to_def(&draft).1,
        "the Injury tab answers its own draft and nothing else. A reply that folded the \
         weighting draft in beside it would not parse as an InjuryDef",
    );
    Ok(())
}

#[test]
fn a_terrain_draft_that_will_not_convert_answers_not_savable() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, "Terrain")?;
    client.exchange(
        &mut app,
        &run_editor(EDITOR_SET_FIELD, "(field: Terrain(Kind(Emplacement)))"),
    )?;

    let body = read_draft(&mut app, &mut client)?;
    assert_eq!(
        body.outcome,
        DraftOutcomeRow::NotSavable(SaveFaultRow::MissingMountedWeapon),
        "an Emplacement with no mounted weapon is what the save path itself refuses, so the \
         read reports that fault rather than refusing the call or answering empty text",
    );
    Ok(())
}

#[test]
fn draft_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(run_editor(EDITOR_DRAFT, "()"), "the editor.draft run")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "every draft is scoped to Editing, so a Load-pass call is refused rather than creating \
         one",
    );

    let note = unavailable_note(&reply)?;
    assert!(
        note.contains("Editing"),
        "the phase is checked before the tab, so a Load-pass refusal names the scene the caller \
         waits for: {note:?}",
    );
    assert!(
        !note.contains("editor.map"),
        "there is no open tab during Load, so the tab note is the wrong answer here. Checking \
         the tab first reaches it, because the mode is absent and reads as Prefab: {note:?}",
    );
    Ok(())
}

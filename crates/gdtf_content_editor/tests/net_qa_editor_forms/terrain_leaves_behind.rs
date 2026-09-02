//! The Terrain form's `leaves_behind` control, written over `editor.set_field`.

use bevy::app::App;
use gdtf_battle_sim::terrain::{
    def::{LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainUuid},
    piece::TerrainGraphicKey,
};
use gdtf_content_editor::EditorMode;
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    draft_reply::{DraftOutcomeRow, DraftReplyRow},
    names::EDITOR_DRAFT,
    outcome::ran_body,
    rows::{FieldRow, TerrainFieldRow},
    setup::{form_tab_app_and_client, set_field, terrain_draft, try_set_field},
    socket::{Client, run_editor},
    support::{TestError, TestResult},
    values::LeavesBehindRow,
};

// The first def key the leaves-behind pick offers, read off the registry the form reads.
fn a_terrain_key(app: &App) -> Result<TerrainUuid, TestError> {
    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        return Err("the terrain registry is what the leaves-behind pick reads".into());
    };
    let mut keys: Vec<TerrainUuid> = registry.defs().map(|(key, _)| *key).collect();
    keys.sort_by_key(|key| (**key).to_string());
    match keys.first() {
        Some(first) => Ok(*first),
        None => Err("the terrain registry holds no def to leave behind".into()),
    }
}

// The first sprite name the leaves-behind pick offers, read off the registry the form reads.
fn a_sprite_name(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<SpriteDefRegistry>() else {
        return Err("the sprite registry is what the leaves-behind pick reads".into());
    };
    let mut names: Vec<String> = registry.keys().map(|name| (**name).clone()).collect();
    names.sort();
    match names.first() {
        Some(first) => Ok(first.clone()),
        None => Err("the sprite registry holds no sprite to leave behind".into()),
    }
}

// What the RON `editor.draft` renders says the save would write for `leaves_behind`.
fn projected_leaves_behind(app: &mut App, client: &mut Client) -> Result<LeavesBehind, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_DRAFT, "()"))?;
    let body: DraftReplyRow = ran_body(&reply, EDITOR_DRAFT)?;
    let DraftOutcomeRow::Ron(text) = &body.outcome else {
        return Err(format!("expected projected RON for the Terrain draft, got {body:?}").into());
    };
    Ok(ron::de::from_str::<TerrainDef>(text)?.leaves_behind)
}

#[test]
fn every_leaves_behind_choice_writes_the_draft_and_the_projected_ron() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let key = a_terrain_key(&app)?;
    let sprite = a_sprite_name(&app)?;

    let piece = set_field(
        &mut app,
        &mut client,
        &format!("(field: Terrain(LeavesBehind(Piece(\"{}\"))))", *key),
    )?;
    assert_eq!(
        piece.field,
        FieldRow::Terrain(TerrainFieldRow::LeavesBehind(LeavesBehindRow::Piece(
            (*key).to_string()
        ))),
    );
    assert_eq!(
        projected_leaves_behind(&mut app, &mut client)?,
        LeavesBehind::Piece(key),
        "the RON editor.draft renders is what the save would write, so the arm has to reach \
         draft_to_terrain_def and not just the draft",
    );

    let named = set_field(
        &mut app,
        &mut client,
        &format!("(field: Terrain(LeavesBehind(Sprite(\"{sprite}\"))))"),
    )?;
    assert_eq!(
        named.field,
        FieldRow::Terrain(TerrainFieldRow::LeavesBehind(LeavesBehindRow::Sprite(
            sprite.clone()
        ))),
    );
    assert_eq!(
        projected_leaves_behind(&mut app, &mut client)?,
        LeavesBehind::Sprite(TerrainGraphicKey::new(sprite)),
    );

    let nothing = set_field(
        &mut app,
        &mut client,
        "(field: Terrain(LeavesBehind(Nothing)))",
    )?;
    assert_eq!(
        nothing.field,
        FieldRow::Terrain(TerrainFieldRow::LeavesBehind(LeavesBehindRow::Nothing)),
    );
    assert_eq!(
        projected_leaves_behind(&mut app, &mut client)?,
        LeavesBehind::Nothing,
    );
    Ok(())
}

#[test]
fn a_leaves_behind_def_key_no_registry_holds_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let before = terrain_draft(&app)?.leaves_behind().clone();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Terrain(LeavesBehind(Piece(\"00000000-0000-4000-8000-00000000dead\"))))",
    )?;
    crate::bad_arguments::bad_arguments_detail(&reply)?;
    assert_eq!(
        terrain_draft(&app)?.leaves_behind(),
        &before,
        "the pick offers no such row, so the refused write left the draft as it was",
    );
    Ok(())
}

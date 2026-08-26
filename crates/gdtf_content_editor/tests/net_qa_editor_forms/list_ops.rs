use bevy::app::App;
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    terrain::{def::TerrainTag, facing::TerrainFacing},
};
use gdtf_content_editor::EditorMode;
use gdtf_content_families::sprites::{SpriteImagePath, SpriteSource};

use crate::{
    bad_arguments::bad_arguments_detail,
    rows::{ListMemberRow, ListRow},
    setup::{
        attachment_draft, form_tab_app_and_client, list_op, set_field, sprite_draft, terrain_draft,
        try_list_op,
    },
    socket::Client,
    sprite_fields::A_SHEET,
    support::{TestError, TestResult},
    values::{EffectRow, FacingRow, SourceRow, TagRow},
};

const TOGGLE_EAST: &str = "(list: EntrySides, op: Toggle(EntrySide(East)))";

const TOGGLE_OPENABLE: &str = "(list: TerrainTags, op: Toggle(TerrainTag(Openable)))";

// The frame sources the world's own draft holds right now.
fn frames(app: &App) -> Result<Vec<SpriteSource>, TestError> {
    let draft = sprite_draft(app)?;
    let Some(animation) = draft.def().animation.as_ref() else {
        return Err("the draft is animated, so it holds an animation".into());
    };
    Ok(animation.frames.clone())
}

fn a_file(name: &str) -> SpriteSource {
    SpriteSource::File(SpriteImagePath::new(name.to_owned()))
}

// A three-frame animation whose frames are all different, so a reorder is visible.
fn three_frames(app: &mut App, client: &mut Client) -> Result<(), TestError> {
    set_field(app, client, A_SHEET)?;
    set_field(app, client, "(field: SpriteAnimated(true))")?;
    set_field(
        app,
        client,
        "(field: SpriteFrame(index: 0, source: File(\"sprites/a.png\")))",
    )?;
    list_op(app, client, "(list: SpriteFrames, op: Add)")?;
    set_field(
        app,
        client,
        "(field: SpriteFrame(index: 1, source: File(\"sprites/b.png\")))",
    )?;
    list_op(app, client, "(list: SpriteFrames, op: Add)")?;
    set_field(
        app,
        client,
        "(field: SpriteFrame(index: 2, source: File(\"sprites/c.png\")))",
    )?;
    Ok(())
}

#[test]
fn a_list_op_adds_then_removes_an_attachment_effect() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Attachment)?;
    assert!(
        attachment_draft(&app)?.effects().is_empty(),
        "a fresh Attachment draft is cosmetic, or the add below would prove nothing",
    );

    let added = list_op(&mut app, &mut client, "(list: AttachmentEffects, op: Add)")?;
    assert_eq!(added.list, ListRow::AttachmentEffects);
    assert_eq!(
        added.members,
        vec![ListMemberRow::AttachmentEffect(EffectRow::Aim(0.0))],
        "Add seeds the form's own default effect, so a handler seeding another template fails \
         here",
    );
    assert_eq!(
        attachment_draft(&app)?.effects(),
        [AttachmentEffect::Aim(AimDelta::new(0.0))],
        "the world's own draft holds the added effect on the frame that answered",
    );

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: AttachmentEffects, op: Remove(0))",
    )?;
    assert!(
        removed.members.is_empty(),
        "removing the one effect leaves the cosmetic identity, got {:?}",
        removed.members,
    );
    assert!(
        attachment_draft(&app)?.effects().is_empty(),
        "the world's own draft is empty again on the frame that answered",
    );
    Ok(())
}

#[test]
fn sprite_frames_move_up_and_down_and_the_world_agrees() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    three_frames(&mut app, &mut client)?;
    assert_eq!(
        frames(&app)?,
        vec![
            a_file("sprites/a.png"),
            a_file("sprites/b.png"),
            a_file("sprites/c.png")
        ],
        "the case starts from three frames that differ, or a reorder would prove nothing",
    );

    let moved = list_op(&mut app, &mut client, "(list: SpriteFrames, op: MoveUp(2))")?;
    assert_eq!(moved.list, ListRow::SpriteFrames);
    assert_eq!(
        moved.members,
        vec![
            ListMemberRow::SpriteFrame(SourceRow::File("sprites/a.png".to_owned())),
            ListMemberRow::SpriteFrame(SourceRow::File("sprites/c.png".to_owned())),
            ListMemberRow::SpriteFrame(SourceRow::File("sprites/b.png".to_owned())),
        ],
        "MoveUp swaps the frame with the one above it, and the reply reads the whole list back",
    );
    assert_eq!(
        frames(&app)?,
        vec![
            a_file("sprites/a.png"),
            a_file("sprites/c.png"),
            a_file("sprites/b.png")
        ],
        "the world's own draft holds the new order on the frame that answered",
    );

    let moved = list_op(
        &mut app,
        &mut client,
        "(list: SpriteFrames, op: MoveDown(0))",
    )?;
    assert_eq!(
        moved.members,
        vec![
            ListMemberRow::SpriteFrame(SourceRow::File("sprites/c.png".to_owned())),
            ListMemberRow::SpriteFrame(SourceRow::File("sprites/a.png".to_owned())),
            ListMemberRow::SpriteFrame(SourceRow::File("sprites/b.png".to_owned())),
        ],
        "MoveDown swaps the frame with the one below it",
    );
    assert_eq!(
        frames(&app)?,
        vec![
            a_file("sprites/c.png"),
            a_file("sprites/a.png"),
            a_file("sprites/b.png")
        ],
        "the world's own draft holds that order too",
    );
    Ok(())
}

#[test]
fn a_toggle_adds_the_entry_side_then_takes_it_back_off() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    set_field(&mut app, &mut client, "(field: TerrainKind(Emplacement))")?;
    assert!(
        terrain_draft(&app)?.entry_sides().is_empty(),
        "a fresh Emplacement draft names no entry side, or the toggles below would prove nothing",
    );

    let added = list_op(&mut app, &mut client, TOGGLE_EAST)?;
    assert_eq!(added.list, ListRow::EntrySides);
    assert_eq!(
        added.members,
        vec![ListMemberRow::EntrySide(FacingRow::East)],
        "the reply reads back the list the write left, holding only the toggled side",
    );
    assert_eq!(
        terrain_draft(&app)?.entry_sides(),
        [TerrainFacing::East],
        "the world's own Terrain draft holds the toggled side on the frame that answered",
    );

    let removed = list_op(&mut app, &mut client, TOGGLE_EAST)?;
    assert!(
        removed.members.is_empty(),
        "toggling the same side again takes it back off, got {:?}",
        removed.members,
    );
    assert!(terrain_draft(&app)?.entry_sides().is_empty());
    Ok(())
}

#[test]
fn a_tag_toggle_adds_the_tag_then_takes_it_back_off() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    assert!(
        !terrain_draft(&app)?.has_tag(TerrainTag::Openable),
        "a fresh Terrain draft holds no tag, or the toggles below would prove nothing",
    );

    let added = list_op(&mut app, &mut client, TOGGLE_OPENABLE)?;
    assert_eq!(added.list, ListRow::TerrainTags);
    assert_eq!(
        added.members,
        vec![ListMemberRow::TerrainTag(TagRow::Openable)],
        "the reply reads back the tags the write left",
    );
    assert!(terrain_draft(&app)?.has_tag(TerrainTag::Openable));

    let removed = list_op(&mut app, &mut client, TOGGLE_OPENABLE)?;
    assert!(
        removed.members.is_empty(),
        "a toggle that only ever adds fails here, got {:?}",
        removed.members,
    );
    assert!(!terrain_draft(&app)?.has_tag(TerrainTag::Openable));
    Ok(())
}

#[test]
fn a_positional_op_on_a_toggle_only_list_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Terrain)?;
    let before = terrain_draft(&app)?.tags().to_vec();

    let reply = try_list_op(&mut app, &mut client, "(list: TerrainTags, op: Add)")?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        terrain_draft(&app)?.tags().to_vec(),
        before,
        "the tag row is a row of tick boxes, so an Add leaves it exactly as it was",
    );
    Ok(())
}

#[test]
fn a_toggle_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = crate::load_case::reply_answered_during_load(
        crate::socket::run_editor(crate::names::EDITOR_LIST_OP, TOGGLE_EAST),
        "the editor.list_op run",
    )?;
    assert_eq!(
        crate::outcome::unavailable_code(&reply)?,
        "WrongState",
        "the draft is state-scoped to Editing, so a Load-pass call is refused rather than \
         creating one",
    );
    Ok(())
}

#[test]
fn a_toggle_is_refused_on_the_prefab_tab() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Prefab)?;

    let reply = try_list_op(&mut app, &mut client, TOGGLE_EAST)?;
    assert_eq!(
        crate::outcome::unavailable_code(&reply)?,
        "WrongState",
        "this command edits the open form's draft, and the Prefab tab is the map canvas and \
         holds no draft, so it is refused rather than written through",
    );
    assert!(
        terrain_draft(&app)?.entry_sides().is_empty(),
        "the refused write left no side on the draft",
    );
    Ok(())
}

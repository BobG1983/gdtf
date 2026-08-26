use bevy::app::App;
use gdtf_battle_sim::effects::attachments::{AimDelta, AttachmentEffect};
use gdtf_content_editor::EditorMode;
use gdtf_content_families::sprites::{SpriteImagePath, SpriteSource};

use crate::{
    rows::{ListMemberRow, ListRow},
    setup::{attachment_draft, form_tab_app_and_client, list_op, set_field, sprite_draft},
    socket::Client,
    sprite_fields::A_SHEET,
    support::{TestError, TestResult},
    values::{EffectRow, SourceRow},
};

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

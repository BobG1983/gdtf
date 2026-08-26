use gdtf_content_editor::EditorMode;
use gdtf_content_families::sprites::{SpriteFacing, SpriteImagePath, SpriteSource};

use crate::{
    rows::FieldRow,
    setup::{form_tab_app_and_client, set_field, sprite_draft},
    support::TestResult,
    values::{FacingRow, RectRow, SourceRow},
};

/// A sheet source with a rect the anchor clamp has an edge to clamp against.
pub(crate) const A_SHEET: &str = "(field: SpriteBaseSource(Sheet(sheet: \"sprites/sheet.png\", rect: (x: 0, y: 0, w: 16, h: \
     32))))";

/// The width and height that sheet names.
pub(crate) const SHEET_RECT: RectRow = RectRow { w: 16, h: 32 };

#[test]
fn every_sprite_field_arm_writes_the_draft_the_form_would_write() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;

    let named = set_field(&mut app, &mut client, "(field: SpriteName(\"scav\"))")?;
    assert_eq!(named.field, FieldRow::SpriteName("scav".to_owned()));
    assert_eq!(
        sprite_draft(&app)?.name(),
        "scav",
        "the world's own Sprite draft holds the name on the frame that answered",
    );

    let based = set_field(&mut app, &mut client, A_SHEET)?;
    assert_eq!(
        based.field,
        FieldRow::SpriteBaseSource(SourceRow::Sheet {
            sheet: "sprites/sheet.png".to_owned(),
            rect:  SHEET_RECT,
        }),
        "the reply reads the base source back off the draft",
    );

    let anchored = set_field(&mut app, &mut client, "(field: SpriteAnchorX(5))")?;
    assert_eq!(anchored.field, FieldRow::SpriteAnchorX(5));
    let anchored = set_field(&mut app, &mut client, "(field: SpriteAnchorY(6))")?;
    assert_eq!(anchored.field, FieldRow::SpriteAnchorY(6));

    let overridden = set_field(
        &mut app,
        &mut client,
        "(field: SpriteFacingOverride(facing: North, source: Some(File(\"sprites/n.png\"))))",
    )?;
    assert_eq!(
        overridden.field,
        FieldRow::SpriteFacingOverride {
            facing: FacingRow::North,
            source: Some(SourceRow::File("sprites/n.png".to_owned())),
        },
    );

    let draft = sprite_draft(&app)?;
    assert_eq!(
        (*draft.def().anchor.x, *draft.def().anchor.y),
        (5, 6),
        "the world's own draft holds the anchor both axes were written to",
    );
    assert_eq!(
        draft.facing_override(SpriteFacing::North),
        Some(&SpriteSource::File(SpriteImagePath::new(
            "sprites/n.png".to_owned()
        ))),
        "the world's own draft holds the facing override the write named",
    );

    let cleared = set_field(
        &mut app,
        &mut client,
        "(field: SpriteFacingOverride(facing: North, source: None))",
    )?;
    assert_eq!(
        cleared.field,
        FieldRow::SpriteFacingOverride {
            facing: FacingRow::North,
            source: None,
        },
    );
    assert_eq!(
        sprite_draft(&app)?.facing_override(SpriteFacing::North),
        None,
        "a None source clears the override, the way unticking the facing does",
    );
    Ok(())
}

#[test]
fn the_animation_arms_write_the_draft_once_the_gate_is_open() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    set_field(&mut app, &mut client, A_SHEET)?;

    let switched = set_field(&mut app, &mut client, "(field: SpriteAnimated(true))")?;
    assert_eq!(switched.field, FieldRow::SpriteAnimated(true));
    assert!(
        sprite_draft(&app)?.is_animated(),
        "the world's own draft is animated on the frame that answered",
    );

    let paced = set_field(&mut app, &mut client, "(field: SpriteFps(12.0))")?;
    assert_eq!(paced.field, FieldRow::SpriteFps(12.0));

    let framed = set_field(
        &mut app,
        &mut client,
        "(field: SpriteFrame(index: 0, source: File(\"sprites/a.png\")))",
    )?;
    assert_eq!(
        framed.field,
        FieldRow::SpriteFrame {
            index:  0,
            source: SourceRow::File("sprites/a.png".to_owned()),
        },
    );

    let draft = sprite_draft(&app)?;
    let Some(animation) = draft.def().animation.as_ref() else {
        return Err("the draft is animated, so it holds an animation".into());
    };
    assert!(
        (*animation.fps - 12.0).abs() < f32::EPSILON,
        "the world's own draft holds the fps that was written, got {}",
        *animation.fps,
    );
    assert_eq!(
        animation.frames.first(),
        Some(&SpriteSource::File(SpriteImagePath::new(
            "sprites/a.png".to_owned()
        ))),
        "the world's own draft holds the frame source that was written",
    );
    Ok(())
}

#[test]
fn set_field_clamps_the_sprite_anchor_to_the_sheet_rect_and_reads_the_stored_value_back()
-> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    set_field(&mut app, &mut client, A_SHEET)?;

    let anchored = set_field(&mut app, &mut client, "(field: SpriteAnchorX(999))")?;
    assert_eq!(
        anchored.field,
        FieldRow::SpriteAnchorX(SHEET_RECT.w),
        "the anchor clamps to the sheet rect's width by design, and the reply carries the value \
         as stored rather than the value that was asked for",
    );
    assert_eq!(
        *sprite_draft(&app)?.def().anchor.x,
        SHEET_RECT.w,
        "the world's own draft holds the clamped anchor, so a handler that range-checked the \
         anchor instead of clamping it fails here",
    );
    Ok(())
}

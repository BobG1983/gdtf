use bevy::app::App;

use crate::{
    mcp_editor_authoring::{
        rows::{FieldRow, ListMemberRow, ListRow, NewOutcomeRow, SourceRow, SpriteFieldRow},
        setup::{list_op, new_draft, set_field, set_mode},
    },
    mcp_shared::{mirror::ModeRow, socket::Client, support::TestResult},
};

/// The name this session authors its sprite under.
pub(crate) const AUTHORED_NAME: &str = "qa_authoring_scav";

/// The image the base source and the first animation frame point at.
pub(crate) const BASE_IMAGE: &str = "sprites/qa_authoring_base.png";

/// The image the second animation frame is rewritten to point at.
pub(crate) const SECOND_FRAME_IMAGE: &str = "sprites/qa_authoring_b.png";

/// Where the anchor's x is written to, which no File source clamps.
pub(crate) const ANCHOR_X: u32 = 5;

// The wire's own reading of one file-backed source.
fn a_file(path: &str) -> SourceRow {
    SourceRow::File(path.to_owned())
}

/// Open the Sprite tab on a blank draft, then write every field and list the session authors.
pub(crate) fn author_the_sprite(app: &mut App, client: &mut Client) -> TestResult {
    let opened = set_mode(app, client, "(mode: Sprite)")?;
    assert_eq!(
        opened.mode,
        ModeRow::Sprite,
        "the session authors the Sprite form, so its tab must be the one the reply names",
    );

    let blanked = new_draft(app, client, "(mode: Sprite)")?;
    assert_eq!(
        blanked.outcome,
        NewOutcomeRow::Blanked,
        "the session starts from the blank draft the form's New button builds, so every write \
         below is the only thing in the file it saves",
    );

    let named = set_field(
        app,
        client,
        &format!("(field: Sprite(Name(\"{AUTHORED_NAME}\")))"),
    )?;
    assert_eq!(named.mode, ModeRow::Sprite);
    assert_eq!(
        named.field,
        FieldRow::Sprite(SpriteFieldRow::Name(AUTHORED_NAME.to_owned())),
        "the save derives its file name from the draft's name, so the write must read back",
    );

    let based = set_field(
        app,
        client,
        &format!("(field: Sprite(BaseSource(File(\"{BASE_IMAGE}\"))))"),
    )?;
    assert_eq!(
        based.field,
        FieldRow::Sprite(SpriteFieldRow::BaseSource(a_file(BASE_IMAGE))),
    );

    let anchored = set_field(
        app,
        client,
        &format!("(field: Sprite(AnchorX({ANCHOR_X})))"),
    )?;
    assert_eq!(
        anchored.field,
        FieldRow::Sprite(SpriteFieldRow::AnchorX(ANCHOR_X)),
    );

    let switched = set_field(app, client, "(field: Sprite(Animated(true)))")?;
    assert_eq!(
        switched.field,
        FieldRow::Sprite(SpriteFieldRow::Animated(true)),
        "the frame list is drawn only while animation is on, so the list write below needs it",
    );

    let added = list_op(app, client, "(list: SpriteFrames, op: Add)")?;
    assert_eq!(added.mode, ModeRow::Sprite);
    assert_eq!(added.list, ListRow::SpriteFrames);
    assert_eq!(
        added.members,
        vec![
            ListMemberRow::SpriteFrame(a_file(BASE_IMAGE)),
            ListMemberRow::SpriteFrame(a_file(BASE_IMAGE)),
        ],
        "switching animation on seeds one frame from the base source, and Add appends a copy of \
         the last one",
    );

    let framed = set_field(
        app,
        client,
        &format!("(field: Sprite(Frame(index: 1, source: File(\"{SECOND_FRAME_IMAGE}\"))))"),
    )?;
    assert_eq!(
        framed.field,
        FieldRow::Sprite(SpriteFieldRow::Frame {
            index:  1,
            source: a_file(SECOND_FRAME_IMAGE),
        }),
    );
    Ok(())
}

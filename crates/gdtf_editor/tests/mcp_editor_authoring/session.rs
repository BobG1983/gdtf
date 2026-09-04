use std::{fs, path::PathBuf};

use gdtf_content_families::sprites::{SpriteDef, SpriteImagePath, SpriteSource};
use gdtf_editor::EditorMcpAssetsRoot;
use tempfile::TempDir;

use crate::{
    harness::editing_app_and_client,
    mirror::ModeRow,
    rows::{LastSaveOutcomeRow, SaveOutcomeRow},
    setup::{draft_refusal, draft_ron, last_save_rows, save},
    support::TestResult,
    writes::{ANCHOR_X, AUTHORED_NAME, BASE_IMAGE, SECOND_FRAME_IMAGE, author_the_sprite},
};

/// The one mode `editor.last_save` files this session's save under.
const ONLY_SPRITE: &str = "(mode: Some(Sprite))";

// The same source as the sim's own type, which is what the saved file parses back to.
fn a_source(path: &str) -> SpriteSource {
    SpriteSource::File(SpriteImagePath::new(path.to_owned()))
}

#[test]
fn one_authoring_session_writes_a_sprite_saves_it_and_reads_every_write_back() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let root = TempDir::new()?;
    app.world_mut()
        .insert_resource(EditorMcpAssetsRoot::new(root.path().to_path_buf()));

    let too_early = draft_refusal(&mut app, &mut client)?;
    assert_eq!(
        too_early, "WrongState",
        "the session opens with the Prefab canvas, which holds no draft, so the readback it \
         ends with is refused until the tab write has run",
    );

    author_the_sprite(&mut app, &mut client)?;
    let outcome = save(&mut app, &mut client, "(mode: Sprite)")?;

    let SaveOutcomeRow::Wrote { path } = outcome else {
        unreachable!("a save into a writable temp root writes a file, got {outcome:?}");
    };
    let written = PathBuf::from(&path);
    assert!(
        written.starts_with(root.path()),
        "the save must land under the QA assets root the test set, never in the repo's own \
         assets tree: `{path}`",
    );
    assert!(
        written
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(AUTHORED_NAME)),
        "the name written over the wire must reach the file name, which was {}",
        written.display(),
    );

    let (read_back_from, projected) = draft_ron(&mut app, &mut client)?;
    assert_eq!(
        read_back_from,
        ModeRow::Sprite,
        "editor.draft reads the active tab and takes no mode argument, so the readback must \
         name the tab the session opened",
    );
    let projected = ron::de::from_str::<SpriteDef>(&projected)?;
    let saved = ron::de::from_str::<SpriteDef>(&fs::read_to_string(&written)?)?;
    assert_eq!(
        projected, saved,
        "editor.draft projects the draft through the same conversion and serializer the save \
         runs, so the readback must be the file, character for character",
    );

    let Some(animation) = saved.animation.as_ref() else {
        unreachable!("the session switched animation on, so the file carries one: {saved:?}");
    };
    assert_eq!(
        *saved.anchor.x, ANCHOR_X,
        "the anchor written over the wire is in the file the save wrote: {saved:?}",
    );
    assert_eq!(
        animation.frames,
        vec![a_source(BASE_IMAGE), a_source(SECOND_FRAME_IMAGE)],
        "both the list write and the frame rewrite are in the file: the seeded frame still names \
         the base source and the appended one names the image it was rewritten to",
    );

    let records = last_save_rows(&mut app, &mut client, ONLY_SPRITE)?;
    let Some(record) = records.iter().find(|row| row.mode == ModeRow::Sprite) else {
        unreachable!("the save just answered with a path, so it left a Sprite row: {records:?}");
    };
    assert_eq!(
        record.outcome,
        LastSaveOutcomeRow::Wrote { path },
        "the record carries the same path the save reply carried, so a QA-driven session reads \
         back through editor.last_save the way the form's own Save button does",
    );
    Ok(())
}

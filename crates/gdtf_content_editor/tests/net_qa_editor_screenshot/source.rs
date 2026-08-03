use gdtf_content_editor::EditorShotSource;

use crate::{harness::headless_editor_app, support::TestResult};

#[test]
fn the_real_editor_app_captures_through_the_shipped_source() -> TestResult {
    let tmp = tempfile::TempDir::new()?;
    let (app, _port) = headless_editor_app(tmp.path().to_path_buf())?;

    let source = app.world().get_resource::<EditorShotSource>();
    assert!(
        matches!(source, Some(EditorShotSource::Offscreen(_))),
        "the real editor app must carry the capture source its QA plugin installs — \
         EditorShotSource::Offscreen since GTW-918, never PrimaryWindow; it carries {source:?}",
    );
    Ok(())
}

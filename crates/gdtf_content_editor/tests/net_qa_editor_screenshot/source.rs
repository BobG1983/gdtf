//! The capture source the REAL editor app ends up with (GTW-917).
//!
//! The in-src test `src/net_qa/screenshot/test/source.rs` pins what `NetQaEditorPlugin`
//! installs on an app carrying that plugin and nothing else, and pins each
//! [`EditorShotSource`] arm to the render target its spawned `Screenshot` carries. This is the
//! same fact one level up, on the REAL editor: [`MapEditorPlugin`] on the no-renderer harness
//! with the editor's own QA plugin on top, which is what the shipped editor binary builds
//! (minus the env gate the plugin's `listening` constructor stands in for). Nothing in that
//! app, and nothing in `harness::pin_tunables`, inserts an [`EditorShotSource`] — so whatever
//! the resource holds is what the plugin installed and what a live capture would read.

use gdtf_content_editor::EditorShotSource;

use crate::{harness::headless_editor_app, support::TestResult};

/// The REAL editor app captures through the source `NetQaEditorPlugin` installs — today
/// [`EditorShotSource::PrimaryWindow`], the window swapchain.
///
/// The VALUE is the point rather than the constant: this pins what the shipped editor
/// captures through, so it cannot change silently. GTW-918 gives the editor an offscreen
/// render target and flips this assertion to
/// [`Offscreen`](EditorShotSource::Offscreen) — deliberately, with this test changing with it.
#[test]
fn the_real_editor_app_captures_through_the_shipped_source() -> TestResult {
    let tmp = tempfile::TempDir::new()?;
    let (app, _port) = headless_editor_app(tmp.path().to_path_buf())?;

    let source = app.world().get_resource::<EditorShotSource>();
    assert!(
        matches!(source, Some(EditorShotSource::PrimaryWindow)),
        "the real editor app must carry the capture source its QA plugin installs — today \
         EditorShotSource::PrimaryWindow, which GTW-918 changes to Offscreen; it carries \
         {source:?}",
    );
    Ok(())
}

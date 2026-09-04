use gdtf_editor::{EditorMcpAssetsRoot, MapEditorPlugin};
use gdtf_test_utils::GdtfUiTestAppBuilder;

use crate::harness::advance_to_editing;

#[test]
fn the_editor_holds_the_one_save_root_with_no_qa_listener_bound() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    advance_to_editing(&mut app);

    let Some(root) = app.world().get_resource::<EditorMcpAssetsRoot>() else {
        unreachable!(
            "the Save weighting button writes through the root-taking writer, so the editor \
             holds a root even when the QA channel never bound a listener",
        );
    };
    assert!(
        root.is_dir(),
        "the root the Save weighting button writes under must be a directory that exists, got \
         `{}`",
        root.display(),
    );
}

//! Thin wrapper around the GDTF content-editor app, the entry point for the editor binary.

use gdtf_content_editor::MapEditorApp;

fn main() {
    MapEditorApp::new().run();
}

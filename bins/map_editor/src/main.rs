//! Thin wrapper around the GDTF map-editor app, the entry point for the editor binary.

use gdtf_editor::MapEditorApp;

fn main() {
    MapEditorApp::new().run();
}

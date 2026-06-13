//! Thin wrapper around the GDTF app, which is the main entry point for the application.

use gdtf_app::GdtfApp;

fn main() {
    GdtfApp::new().run();
}

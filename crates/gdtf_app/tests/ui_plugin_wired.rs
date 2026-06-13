//! GTW-116 regression: the headless app built by the real test-support wiring
//! has [`UiPlugin`] registered — proving the registration path actually adds the
//! UI seam, not merely that `gdtf_ui` compiles.
//!
//! This is a *pin-discriminating* test. [`gdtf_app::test_support::register_headless`]
//! mirrors [`gdtf_app::GdtfApp`]; both add `UiPlugin`. If a refactor dropped the
//! `UiPlugin` registration from that path, [`bevy::app::App::is_plugin_added`]
//! would return `false` and this assertion would fail. An empty `UiPlugin::build`
//! installs no systems to observe, so plugin-presence is the right assertion.

use gdtf_app::test_support::UiPlugin;
use gdtf_test_utils::GdtfTestAppBuilder;

/// The headless app — wired exactly as the real `GdtfApp` is — registers
/// [`UiPlugin`].
///
/// Pin: this fails if the `UiPlugin` registration is removed from
/// `register_headless` (and, by mirror, from `GdtfApp::add_plugins`), turning a
/// silently-unwired UI seam into a red test instead of a runtime surprise.
#[test]
fn headless_app_registers_ui_plugin() {
    let app = GdtfTestAppBuilder::new().default_start().build();

    assert!(
        app.is_plugin_added::<UiPlugin>(),
        "the headless test-support wiring must register UiPlugin, mirroring GdtfApp",
    );
}

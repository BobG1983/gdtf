//! Headless test wiring must register UiPlugin like the real app.
use gdtf_app::test_support::UiPlugin;
use gdtf_test_utils::GdtfTestAppBuilder;

#[test]
fn headless_app_registers_ui_plugin() {
    let app = GdtfTestAppBuilder::new().default_start().build();

    assert!(
        app.is_plugin_added::<UiPlugin>(),
        "the headless test-support wiring must register UiPlugin, mirroring GdtfApp",
    );
}

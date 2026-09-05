//! Headless test wiring must register `UiPlugin` like the real app.
use cobalt_test_utils::MinimalTestAppBuilder;
use gdtf_game::test_support::UiPlugin;

#[test]
fn headless_app_registers_ui_plugin() {
    let app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .default_start()
        .build();

    assert!(
        app.is_plugin_added::<UiPlugin>(),
        "the headless test-support wiring must register UiPlugin, mirroring GdtfApp",
    );
}

mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::MenuScenePlugin;

mod components;
// Re-exported ONLY for the external integration tests (gdtf_app::test_support);
// internal code names the markers via the direct `components::` path, so this
// re-export is test-support-only — gating it keeps the binary build (no
// test-support) free of an unused-import warning. (GTW-145)
#[cfg(feature = "test-support")]
crate::support_use! {
    crate::scenes::running::menu::components::{
        BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    };
}

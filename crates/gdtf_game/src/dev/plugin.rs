use bevy::prelude::*;

pub(crate) struct DevAffordancesPlugin;

impl Plugin for DevAffordancesPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(all(feature = "dev_tools", not(feature = "headless_test")))]
        {
            use bevy::render::RenderPlugin;
            use bevy_egui::{EguiGlobalSettings, EguiPlugin};

            if app.is_plugin_added::<RenderPlugin>() {
                app.add_plugins(EguiPlugin::default());
                app.insert_resource(EguiGlobalSettings {
                    auto_create_primary_context: false,
                    ..default()
                });
                app.add_systems(Update, super::egui_context::bind_primary_egui_context);
            }
        }
        #[cfg(feature = "dev_tools")]
        app.add_plugins(super::procgen_stepper::ProcgenStepperPlugin::with_enabled(
            false,
        ));
        #[cfg(feature = "mcp")]
        app.add_plugins(super::mcp::McpPlugin::from_env());
        #[cfg(not(any(feature = "dev_tools", feature = "mcp")))]
        let _ = app;
    }
}

//! Core wiring and app logic for GDTF.

use bevy::prelude::*;
use gdtf_ui::UiPlugin;

use crate::{scenes::ScenesPlugin, states::AppState};

/// Main entry point for the GDTF application.
pub struct GdtfApp(App);

impl GdtfApp {
    /// Crate a new GDTF application instance.
    #[must_use]
    pub fn new() -> Self {
        let app = Self(App::new());

        app.add_bevy_plugins().add_states().add_plugins()
    }

    /// Run the GDTF application.
    pub fn run(mut self) {
        self.0.run();
    }

    #[must_use]
    fn add_bevy_plugins(mut self) -> Self {
        self.0.add_plugins(DefaultPlugins);
        self
    }

    #[must_use]
    fn add_plugins(mut self) -> Self {
        self.0.add_plugins(ScenesPlugin);
        self.0.add_plugins(UiPlugin);
        self
    }

    #[must_use]
    fn add_states(mut self) -> Self {
        self.0.init_state::<AppState>();
        self
    }
}

impl Default for GdtfApp {
    fn default() -> Self {
        Self::new()
    }
}

//! Presenter mode selection and entry plugins.

use bevy::prelude::*;

use super::topdown::TopDownRendererPlugin;

/// Which renderer the battle presenter runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePresenterMode {
    /// Orthographic top-down tiles.
    TopDown,
    /// Isometric placeholder (no systems yet).
    Iso,
}

/// App plugin that installs the chosen renderer.
pub struct BattlePresenterPlugin {
    mode: BattlePresenterMode,
}

impl BattlePresenterPlugin {
    /// Build a presenter plugin for `mode`.
    #[must_use]
    pub const fn new(mode: BattlePresenterMode) -> Self {
        Self { mode }
    }

    /// The mode this plugin was constructed with.
    #[must_use]
    pub const fn mode(&self) -> BattlePresenterMode {
        self.mode
    }
}

impl Default for BattlePresenterPlugin {
    fn default() -> Self {
        Self::new(BattlePresenterMode::TopDown)
    }
}

impl Plugin for BattlePresenterPlugin {
    fn build(&self, app: &mut App) {
        match self.mode {
            BattlePresenterMode::TopDown => {
                app.add_plugins(TopDownRendererPlugin);
            }
            BattlePresenterMode::Iso => {
                app.add_plugins(IsoRendererPlugin);
            }
        }
    }
}

/// Placeholder iso renderer; no systems yet.
pub struct IsoRendererPlugin;

impl Plugin for IsoRendererPlugin {
    fn build(&self, _app: &mut App) {}
}

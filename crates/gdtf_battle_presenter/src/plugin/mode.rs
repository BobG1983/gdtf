use bevy::prelude::*;

use super::topdown::TopDownRendererPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePresenterMode {
        TopDown,
            Iso,
}

pub struct BattlePresenterPlugin {
        mode: BattlePresenterMode,
}

impl BattlePresenterPlugin {
        #[must_use]
    pub const fn new(mode: BattlePresenterMode) -> Self {
        Self { mode }
    }

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

pub struct IsoRendererPlugin;

impl Plugin for IsoRendererPlugin {
    fn build(&self, _app: &mut App) {}
}

//! The presenter mode selector — which battle renderer the seam builds (the real
//! top-down renderer or the iso stub).

use bevy::prelude::*;

use super::topdown::TopDownRendererPlugin;

/// Which battle renderer the [`BattlePresenterPlugin`] builds.
///
/// A domain value (the named presentation mode), so it is a real type rather than
/// a bare primitive. Exhaustive: exactly the two variants the design supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePresenterMode {
    /// The top-down 16×16 sprite renderer — the shipping presenter.
    TopDown,
    /// The isometric renderer — an intentional stub for the whole of GTW-48 (the
    /// real iso renderer is GTW-49 / GTW-10).
    Iso,
}

/// The battle presenter seam: selects and builds one battle renderer per its
/// [`BattlePresenterMode`].
///
/// Added by `GameBattleScapeScenePlugin` so its `build` runs when the scene plugins
/// register. On `build` it adds the renderer plugin chosen by [`Self::mode`]:
/// [`TopDownRendererPlugin`](super::topdown::TopDownRendererPlugin) for
/// [`BattlePresenterMode::TopDown`], the [`IsoRendererPlugin`] stub for
/// [`BattlePresenterMode::Iso`].
pub struct BattlePresenterPlugin {
    /// The renderer this plugin builds.
    mode: BattlePresenterMode,
}

impl BattlePresenterPlugin {
    /// Construct a presenter that builds the renderer for `mode`.
    #[must_use]
    pub const fn new(mode: BattlePresenterMode) -> Self {
        Self { mode }
    }

    /// The [`BattlePresenterMode`] this plugin builds.
    #[must_use]
    pub const fn mode(&self) -> BattlePresenterMode {
        self.mode
    }
}

impl Default for BattlePresenterPlugin {
    /// The shipping default: the top-down renderer.
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

/// The isometric renderer plugin — a no-op stub for the whole of GTW-48.
///
/// Selected by [`BattlePresenterMode::Iso`]. It stays empty for the whole epic; the
/// real iso renderer is GTW-49 / GTW-10.
pub struct IsoRendererPlugin;

impl Plugin for IsoRendererPlugin {
    fn build(&self, _app: &mut App) {}
}

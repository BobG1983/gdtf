//! Presentation layer for GDTFs turn based battle system.
//!
//! This is the VIEW that mirrors the authoritative, render-free combat sim
//! (`gdtf_battle_sim`). The dependency is strictly one-way: the presenter reads
//! sim state and renders it; the sim never reads the presenter.
//!
//! GTW-215 (the first GTW-48 slice) stands up the empty *home* every later slice
//! plugs into. It lands the [`BattlePresenterPlugin`] seam and the
//! [`BattlePresenterMode`] selector between the real CP437 renderer
//! ([`Cp437RendererPlugin`], deliberately empty this slice) and a no-op iso stub
//! ([`IsoRendererPlugin`], a placeholder for the future iso renderer, GTW-49 /
//! GTW-10). It spawns no camera (S2), loads no atlas (S3), draws no glyph
//! (S4/S5/S6), reads no sim type by name yet, and touches no input (S7/S8). It
//! adds ZERO sim lifecycle plumbing (all landed in E10) — it only compiles, links,
//! and adds the (empty) plugin so later slices have a wired seam.
//!
//! GTW-216 (the S2 slice) adds the SHARED world-camera lifecycle in
//! [`mod@world_camera`]: the [`WorldCamera`]-marked `Camera2d` spawned/despawned on the
//! `GameState::BattleScape` boundary, rendering beneath the GTW-120 UI camera on its own
//! [`WORLD_RENDER_LAYER`]. It adds no glyph draw, atlas, or sim read.

use bevy::prelude::*;

pub mod world_camera;

pub use world_camera::{WORLD_RENDER_LAYER, WorldCamera, despawn_world_camera, spawn_world_camera};

/// Which battle renderer the [`BattlePresenterPlugin`] builds.
///
/// A domain value (the named presentation mode), so it is a real type rather than
/// a bare primitive. Exhaustive: exactly the two variants the design supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePresenterMode {
    /// The CP437 / Cogmind-style sprite-glyph renderer — the shipping presenter.
    Cp437,
    /// The isometric renderer — an intentional stub for the whole of GTW-48 (the
    /// real iso renderer is GTW-49 / GTW-10).
    Iso,
}

/// The battle presenter seam: selects and builds one battle renderer per its
/// [`BattlePresenterMode`].
///
/// Added by `GameBattleScapeScenePlugin` so its `build` runs when the scene plugins
/// register. On `build` it adds the renderer plugin chosen by [`Self::mode`]:
/// [`Cp437RendererPlugin`] for [`BattlePresenterMode::Cp437`], the
/// [`IsoRendererPlugin`] stub for [`BattlePresenterMode::Iso`].
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
    /// The shipping default: the CP437 renderer.
    fn default() -> Self {
        Self::new(BattlePresenterMode::Cp437)
    }
}

impl Plugin for BattlePresenterPlugin {
    fn build(&self, app: &mut App) {
        match self.mode {
            BattlePresenterMode::Cp437 => {
                app.add_plugins(Cp437RendererPlugin);
            }
            BattlePresenterMode::Iso => {
                app.add_plugins(IsoRendererPlugin);
            }
        }
    }
}

/// Marker resource the [`Cp437RendererPlugin`] inserts on `build`.
///
/// Its presence in the world is the test-observable proof that the CP437 renderer
/// plugin's `build` actually ran (AC1/AC3 assert it present, AC2 asserts it absent
/// in the iso-mode app). A framework type (`Resource`), so it is exempt from the
/// no-bare-types rule.
#[derive(Resource)]
pub struct Cp437RendererActive;

/// The real CP437 / Cogmind-style battle renderer plugin.
///
/// Deliberately EMPTY this slice apart from the [`Cp437RendererActive`] marker —
/// later GTW-48 slices add its camera (S2), atlas (S3), and draw systems (S4/S5/S6)
/// behind it. Kept as the CP437-specific home so the future iso swap replaces only
/// this renderer plugin, never shared draw logic.
pub struct Cp437RendererPlugin;

impl Plugin for Cp437RendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Cp437RendererActive);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// AC1 — the default presenter selects `Cp437`, builds without panic under
    /// `MinimalPlugins`, and its CP437 renderer inserts the marker resource.
    #[test]
    fn default_presenter_selects_cp437_and_inserts_the_marker() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(BattlePresenterPlugin::default());
        app.update();

        assert_eq!(
            BattlePresenterPlugin::default().mode(),
            BattlePresenterMode::Cp437,
            "the default presenter mode must be Cp437",
        );
        assert!(
            app.world().get_resource::<Cp437RendererActive>().is_some(),
            "the default (Cp437) presenter must insert the Cp437RendererActive marker",
        );
    }

    /// AC2 — the Cp437-mode app carries the marker; the Iso-mode app does not
    /// (proving the mode switch actually selects the iso branch and the CP437
    /// renderer did not run). Neither app panics on `update()`.
    #[test]
    fn mode_switch_selects_the_named_renderer_branch() {
        let mut cp437_app = App::new();
        cp437_app
            .add_plugins(MinimalPlugins)
            .add_plugins(BattlePresenterPlugin::new(BattlePresenterMode::Cp437));
        cp437_app.update();

        let mut iso_app = App::new();
        iso_app
            .add_plugins(MinimalPlugins)
            .add_plugins(BattlePresenterPlugin::new(BattlePresenterMode::Iso));
        iso_app.update();

        assert!(
            cp437_app
                .world()
                .get_resource::<Cp437RendererActive>()
                .is_some(),
            "the Cp437-mode presenter must carry the Cp437RendererActive marker",
        );
        assert!(
            iso_app
                .world()
                .get_resource::<Cp437RendererActive>()
                .is_none(),
            "the Iso-mode presenter must NOT carry the Cp437 marker — the iso branch ran, not CP437",
        );
    }
}

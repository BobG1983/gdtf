//! Presentation layer for GDTFs turn based battle system.
//!
//! This is the VIEW that mirrors the authoritative, render-free combat sim
//! (`gdtf_battle_sim`). The dependency is strictly one-way: the presenter reads
//! sim state and renders it; the sim never reads the presenter.
//!
//! GTW-215 (the first GTW-48 slice) stands up the empty *home* every later slice
//! plugs into. It lands the [`BattlePresenterPlugin`] seam and the
//! [`BattlePresenterMode`] selector between the real top-down renderer
//! ([`TopDownRendererPlugin`]) and a no-op iso stub ([`IsoRendererPlugin`], a
//! placeholder for the future iso renderer, GTW-49 / GTW-10). It spawned no camera
//! (S2), loaded no atlas (S3), drew no sprite (S4/S5/S6), read no sim type by name,
//! and touched no input (S7/S8).
//!
//! GTW-216 (the S2 slice) adds the SHARED world-camera lifecycle in
//! [`mod@world_camera`]: the [`WorldCamera`]-marked `Camera2d` spawned/despawned on the
//! `GameState::BattleScape` boundary, rendering beneath the GTW-120 UI camera on its own
//! [`WORLD_RENDER_LAYER`]. It adds no sprite draw, atlas, or sim read.
//!
//! GTW-217 (the S3 slice) renames the renderer surface to `TopDown*`
//! (the dead CP437 8×8-glyph approach is replaced by the landed role-separated 16×16
//! top-down sprite set, GTW-224) and stands up the px/coordinate bridge in
//! [`mod@topdown`]: the [`CELL_PX`] cell-size const, the [`cell_to_world`] sim→view
//! projection, and the role-keyed [`TopDownAtlases`] resource loaded ONCE from the
//! three render sheets (terrain / characters / effects) by [`TopDownRendererPlugin`].
//! It still spawns NO sprite and draws NOTHING — that is S4/S5/S6.

use bevy::prelude::*;

pub mod topdown;
pub mod world_camera;

pub use topdown::{
    CELL_PX, SheetAtlas, SheetRole, TopDownAtlases, cell_to_world, load_topdown_atlases,
};
pub use world_camera::{WORLD_RENDER_LAYER, WorldCamera, despawn_world_camera, spawn_world_camera};

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
/// [`TopDownRendererPlugin`] for [`BattlePresenterMode::TopDown`], the
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

/// Marker resource the [`TopDownRendererPlugin`] inserts on `build`.
///
/// Its presence in the world is the test-observable proof that the top-down renderer
/// plugin's `build` actually ran (the default-mode and integration tests assert it
/// present, the iso-mode test asserts it absent). A framework type (`Resource`), so
/// it is exempt from the no-bare-types rule.
#[derive(Resource)]
pub struct TopDownRendererActive;

/// The real top-down 16×16 sprite battle renderer plugin.
///
/// On `build` it inserts the [`TopDownRendererActive`] marker and schedules
/// [`load_topdown_atlases`] in [`Startup`] so the role-keyed [`TopDownAtlases`]
/// resource is built ONCE and present before the S4/S5/S6 draw systems run. It still
/// spawns NO sprite this slice — later GTW-48 slices add the draw systems (S4/S5/S6)
/// behind it. Kept as the top-down-specific home so the future iso swap replaces only
/// this renderer plugin, never shared draw logic.
///
/// The atlas-load system is gated on [`Assets<TextureAtlasLayout>`] existing so the
/// plugin still builds under `MinimalPlugins` (the renamed S1 unit tests add no
/// asset stack): without the sprite/asset plugins that collection is absent and the
/// load is a no-op rather than a param-validation failure. Under `DefaultPlugins`
/// (the app and the AC4 load harness) `SpritePlugin`'s `TextureAtlasPlugin` provides
/// it, so the load runs for real.
pub struct TopDownRendererPlugin;

impl Plugin for TopDownRendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TopDownRendererActive).add_systems(
            Startup,
            load_topdown_atlases.run_if(resource_exists::<Assets<bevy::image::TextureAtlasLayout>>),
        );
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

    /// AC1 — the default presenter selects `TopDown`, builds without panic under
    /// `MinimalPlugins`, and its top-down renderer inserts the marker resource.
    #[test]
    fn default_presenter_selects_topdown_and_inserts_the_marker() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(BattlePresenterPlugin::default());
        app.update();

        assert_eq!(
            BattlePresenterPlugin::default().mode(),
            BattlePresenterMode::TopDown,
            "the default presenter mode must be TopDown",
        );
        assert!(
            app.world()
                .get_resource::<TopDownRendererActive>()
                .is_some(),
            "the default (TopDown) presenter must insert the TopDownRendererActive marker",
        );
    }

    /// AC1 — the TopDown-mode app carries the marker; the Iso-mode app does not
    /// (proving the mode switch actually selects the iso branch and the top-down
    /// renderer did not run). Neither app panics on `update()`.
    #[test]
    fn mode_switch_selects_the_named_renderer_branch() {
        let mut topdown_app = App::new();
        topdown_app
            .add_plugins(MinimalPlugins)
            .add_plugins(BattlePresenterPlugin::new(BattlePresenterMode::TopDown));
        topdown_app.update();

        let mut iso_app = App::new();
        iso_app
            .add_plugins(MinimalPlugins)
            .add_plugins(BattlePresenterPlugin::new(BattlePresenterMode::Iso));
        iso_app.update();

        assert!(
            topdown_app
                .world()
                .get_resource::<TopDownRendererActive>()
                .is_some(),
            "the TopDown-mode presenter must carry the TopDownRendererActive marker",
        );
        assert!(
            iso_app
                .world()
                .get_resource::<TopDownRendererActive>()
                .is_none(),
            "the Iso-mode presenter must NOT carry the TopDown marker — the iso branch ran",
        );
    }
}

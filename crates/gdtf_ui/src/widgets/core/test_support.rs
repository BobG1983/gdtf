//! Shared in-crate test fixtures for the `widgets/core` widget tests.
//!
//! `gdtf_ui` cannot depend on `gdtf_test_utils` (dependency cycle), so the
//! widget tests build their own minimal headless apps: [`harness`] drives the
//! full [`UiPlugin`](crate::UiPlugin) path, [`scene_app`] supplies only the
//! GTW-322 `bsn!` scene-spawn infrastructure, [`theme`] builds a [`GdtfTheme`]
//! through the real resolution path, and [`REMAINING`] / [`LOST`] are the
//! per-role discriminating test colors.

use bevy::{
    MinimalPlugins, asset::AssetPlugin, input::InputPlugin, prelude::*, scene::ScenePlugin,
};

use crate::{
    UiPlugin,
    theme::{GdtfTheme, GdtfThemeSpec},
};

/// A test color tuple, kept distinct per role so asserts discriminate.
pub(crate) const REMAINING: Color = Color::srgb(0.2, 0.8, 0.2);
/// A second distinct test color.
pub(crate) const LOST: Color = Color::srgb(0.8, 0.2, 0.2);

/// Builds the minimal harness used by every HUD-widget test: `MinimalPlugins`
/// (schedules + time), `InputPlugin` (so `Interaction` plumbing is sane),
/// `AssetPlugin` + `ScenePlugin` (GTW-322 — the widget builders spawn via `bsn!`
/// scenes through `Commands::spawn_scene`, which needs the scene/asset
/// infrastructure or it panics on flush), and `UiPlugin` (which registers the
/// GTW-276 driver systems + message buffers).
///
/// The widgets resolve their `bsn!` scenes on `app.world_mut().flush()` (no asset
/// dependencies → `apply_scene` resolves synchronously in the queued command), so
/// the existing tests' single `flush()` still materializes the tree — no extra
/// `app.update()` is needed before asserting.
pub(crate) fn harness() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app
}

/// Builds the headless harness for the widget-builder tests with the scene/asset
/// infrastructure the GTW-322 `bsn!` builders need.
///
/// The builders spawn via [`Commands::spawn_scene`](bevy::scene::CommandsSceneExt::spawn_scene),
/// whose deferred `apply_scene` (run on the next `flush`) reads the `AssetServer` +
/// `Assets<ScenePatch>` resources; without `AssetPlugin` + `ScenePlugin` it panics.
/// `MinimalPlugins` supplies the task pool `AssetPlugin` requires. The widget scenes
/// have no asset dependencies, so they materialize on the existing single `flush()`
/// (no extra `app.update()` is needed before asserting).
pub(crate) fn scene_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app
}

/// Builds a [`GdtfTheme`] through the real resolution path (deserialize the
/// nested spec, then [`GdtfThemeSpec::resolve`]) with a defaulted-font
/// resolver. The button sub-theme's fill is caller-chosen so the paint tests
/// can pin it. The theme newtypes have private fields, so this respects
/// encapsulation while exercising the genuine runtime resolution. Returns the
/// `ron` error so a malformed literal surfaces via `?`.
pub(crate) fn theme(
    button_color: [f32; 4],
    disabled: [f32; 4],
) -> Result<GdtfTheme, ron::error::SpannedError> {
    let [pr, pg, pb, pa] = button_color;
    let [dr, dg, db, da] = disabled;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 2.0, corner_radius: 5.0, \
                  margin: (left: 12.0, right: 12.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, {pa}), disabled: ({dr}, {dg}, {db}, {da}), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
                   text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0, \
                   border_color: (0.20, 0.20, 0.24, 1.0), \
                   border_width: 2.0, corner_radius: 5.0, \
                   margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         title: ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 36.0 ), \
         text:  ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0 ))",
    );
    let spec: GdtfThemeSpec = ron::from_str(&ron)?;
    Ok(spec.resolve(|_| Handle::<Font>::default()))
}

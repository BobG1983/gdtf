use bevy::{MinimalPlugins, input::InputPlugin, prelude::*, scene::ScenePlugin};
use cobalt_test_utils::unwatched_asset_plugin;

use crate::{
    UiPlugin,
    theme::{GdtfTheme, GdtfThemeSpec},
};

pub(crate) const REMAINING: Color = Color::srgb(0.2, 0.8, 0.2);
pub(crate) const LOST: Color = Color::srgb(0.8, 0.2, 0.2);

pub(crate) fn harness() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(unwatched_asset_plugin())
        .add_plugins(ScenePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app
}

pub(crate) fn scene_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app
}

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

use bevy::{MinimalPlugins, prelude::*, scene::ScenePlugin, ui::Interaction};
use cobalt_test_utils::unwatched_asset_plugin;

use super::super::theme_interaction;
use crate::{
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{UiSystems, apply_theme},
};

pub(super) fn theme(
    button_color: [f32; 4],
    hover: [f32; 4],
    pressed: [f32; 4],
) -> Result<GdtfTheme, ron::error::SpannedError> {
    let [pr, pg, pb, pa] = button_color;
    let [hr, hg, hb, ha] = hover;
    let [sr, sg, sb, sa] = pressed;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 2.0, corner_radius: 5.0, \
                  margin: (left: 12.0, right: 12.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, {pa}), disabled: (0.08, 0.08, 0.10, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: ({hr}, {hg}, {hb}, {ha}), pressed: ({sr}, {sg}, {sb}, {sa}), \
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

pub(super) fn app_with_interaction() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_systems(
        Update,
        (
            apply_theme.in_set(UiSystems::ApplyTheme),
            theme_interaction.after(UiSystems::ApplyTheme),
        )
            .run_if(resource_exists::<GdtfTheme>),
    );
    app
}

pub(super) fn set_interaction(app: &mut App, button: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = state;
    }
}

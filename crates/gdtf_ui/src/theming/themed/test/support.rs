use bevy::prelude::*;

use super::super::apply_theme;
use crate::theme::{GdtfTheme, GdtfThemeSpec};

pub(super) fn theme(
    button_color: [f32; 3],
    border: [f32; 3],
    border_w: f32,
    radius: f32,
    button_font_size: f32,
) -> Result<GdtfTheme, ron::error::SpannedError> {
    let [pr, pg, pb] = button_color;
    let [br, bg, bb] = border;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 3.0, corner_radius: 7.0, \
                  margin: (left: 9.0, right: 9.0, top: 4.0, bottom: 4.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, 1.0), disabled: (0.08, 0.08, 0.10, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
                   text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: {button_font_size}, \
                   border_color: ({br}, {bg}, {bb}, 1.0), \
                   border_width: {border_w}, corner_radius: {radius}, \
                   margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         title: ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 36.0 ), \
         text:  ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0 ))",
    );
    let spec: GdtfThemeSpec = ron::from_str(&ron)?;
    Ok(spec.resolve(|_| Handle::<Font>::default()))
}

pub(super) fn app_with_apply_theme() -> App {
    let mut app = App::new();
    app.add_systems(Update, apply_theme.run_if(resource_exists::<GdtfTheme>));
    app
}

pub(super) fn app_with_production_run_condition() -> App {
    let mut app = App::new();
    app.add_systems(
        Update,
        apply_theme.run_if(
            resource_exists::<GdtfTheme>
                .and_then(resource_changed::<GdtfTheme>.or_else(super::super::any_themed_added)),
        ),
    );
    app
}

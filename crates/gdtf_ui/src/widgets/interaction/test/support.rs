//! Shared fixtures for the interaction-layer tests: the caller-parameterized
//! `theme` RON fixture, the production-ordered `app_with_interaction` harness,
//! and the headless `set_interaction` pointer stand-in.

use bevy::{MinimalPlugins, asset::AssetPlugin, prelude::*, scene::ScenePlugin, ui::Interaction};

use super::super::theme_interaction;
use crate::{
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{UiSystems, apply_theme},
};

/// Builds a [`GdtfTheme`] with caller-chosen button resting / hover / pressed
/// colors through the real resolution path (deserialize the nested spec, then
/// [`GdtfThemeSpec::resolve`]) with a defaulted-font resolver. Returns the
/// `ron` error so a malformed literal surfaces via `?` rather than a denied
/// `unwrap`/`panic`.
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

/// Builds a minimal app with the real production schedule: `apply_theme`
/// (in its named set) before `theme_interaction`, both under the live run
/// condition — mirroring [`UiPlugin`](crate::UiPlugin)'s wiring.
///
/// Includes `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` (GTW-322) so the
/// widget `bsn!` builders' `Commands::spawn_scene` resolves on flush instead of
/// panicking on the missing scene/asset resources.
pub(super) fn app_with_interaction() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
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

/// Sets a button's [`Interaction`] in the world (the swap a real pointer
/// would otherwise drive), so the test can exercise the state transitions
/// headlessly.
pub(super) fn set_interaction(app: &mut App, button: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = state;
    }
}

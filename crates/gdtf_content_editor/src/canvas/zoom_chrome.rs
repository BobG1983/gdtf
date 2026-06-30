//! Canvas **zoom chrome** (GTW-500 C3) — the clickable `Zoom n%` readout, its in-place text
//! refresh, and the `0`-key / click zoom-reset.
//!
//! Split from the zoom CORE ([`super::zoom`] — the wheel read + the cell/root re-layout math) so
//! each file owns one concern under the size cap (the [`level_nav`](super::level_nav) /
//! [`center`](super::center) submodule precedent): the core reads input + re-lays-out geometry; this
//! spawns the readout affordance, keeps its text in sync, and handles the reset.

use bevy::prelude::*;
use gdtf_ui::theme::GdtfTheme;

use super::{level_nav::CANVAS_CHROME_Z, zoom::CanvasZoom};
use crate::{CanvasRegion, mode::PrefabModeContent, mode_host::mode_host_under_region};

/// Marker on the canvas chrome's ZOOM readout [`Text`] (C3) — the `Zoom 100%` line
/// [`refresh_zoom_readout`] rewrites whenever the zoom changes; the readout node is itself the
/// clickable zoom-reset [`Button`]. A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ZoomReadout;

/// The press-edge query filter [`reset_zoom_button`] reads — a [`ZoomReadout`] (which is also the
/// clickable reset button) whose [`Interaction`] changed this frame. A named alias to keep the
/// system signature under clippy's `type_complexity` gate.
type PressedReset = (Changed<Interaction>, With<ZoomReadout>);

/// `Update` (in `Editing`): reset the [`CanvasZoom`] to `1.0` on the `0` hotkey or a click of the
/// zoom readout (C3 — a zoom reset affordance).
///
/// `Digit0` resets via the keyboard; clicking the zoom readout (a [`Button`]) resets via the
/// chrome. Written with [`set_if_neq`](DetectChangesMut::set_if_neq) so a reset at the
/// already-unzoomed factor is a no-op. Guarded on the optional state-scoped [`CanvasZoom`]
/// (bevy-traps #1).
pub(crate) fn reset_zoom_button(
    keys: Res<ButtonInput<KeyCode>>,
    pressed: Query<&Interaction, PressedReset>,
    zoom: Option<ResMut<CanvasZoom>>,
) {
    let Some(mut zoom) = zoom else {
        return;
    };
    let clicked = pressed
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Pressed));
    if keys.just_pressed(KeyCode::Digit0) || clicked {
        zoom.set_if_neq(CanvasZoom::reset());
    }
}

/// `Update` (in `Editing`): rewrite the zoom chrome readout `Zoom n%` (C3).
///
/// Mutates the [`ZoomReadout`] [`Text`] in place (the ui-mutate-in-place rule) on a zoom change.
/// Guarded on the optional state-scoped [`CanvasZoom`] (bevy-traps #1).
pub(crate) fn refresh_zoom_readout(
    zoom: Option<Res<CanvasZoom>>,
    mut readouts: Query<&mut Text, With<ZoomReadout>>,
) {
    let Some(zoom) = zoom else {
        return;
    };
    if !zoom.is_changed() {
        return;
    }
    // The factor as a whole-percent label (e.g. 1.0 -> "100%").
    let pct = (**zoom * 100.0).round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "pct is a rounded zoom percent in [25, 400]; the i32 cast cannot wrap"
    )]
    let pct_i = pct as i32;
    let line = format!("Zoom {pct_i}% (0 resets)");
    for mut text in &mut readouts {
        if **text != line {
            (**text).clone_from(&line);
        }
    }
}

/// `OnEnter(Editing)`: spawn the canvas zoom chrome — the clickable `Zoom n%` readout floated at
/// the top-right of the canvas (C3).
///
/// Parented into the region's [`PrefabModeContent`] host (deferred, the level-nav precedent) so it
/// hides in TERRAIN / THEME mode; absolutely positioned at the top-right with the canvas-chrome
/// [`GlobalZIndex`] so it floats over the canvas without disturbing the scroll list. The readout
/// is itself a [`Button`] (clicking it resets the zoom — C3).
pub(crate) fn spawn_zoom_chrome(mut commands: Commands, theme: Res<GdtfTheme>) {
    let text_color = *theme.text.text_color;
    let bg = *theme.panel.color;
    let label = commands
        .spawn((Text::new("Zoom 100% (0 resets)"), TextColor(text_color)))
        .id();
    let readout = commands
        .spawn((
            ZoomReadout,
            Button,
            GlobalZIndex(CANVAS_CHROME_Z),
            BackgroundColor(bg),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                padding: UiRect::axes(Val::Vw(0.4), Val::Vh(0.3)),
                ..default()
            },
        ))
        .add_child(label)
        .id();
    commands.queue(move |world: &mut World| {
        let Some(host) = mode_host_under_region::<CanvasRegion, PrefabModeContent>(world) else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(readout);
        }
    });
}

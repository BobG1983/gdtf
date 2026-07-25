//! The shared SETTING ROW builder: a caption, a pill toggle, and a value readout.
//!
//! Every setting on the Options screen is the same row — a caption, a pill toggle, and an
//! On/Off readout — so it is built once here. The GTW-868 dev-only procgen-stepper row is therefore the
//! sound row's twin by construction, which is what keeps the new control from being a
//! special case (same layout, same theming, same focusable checkbox, so the same
//! focus-navigation treatment applies with no extra work).

use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{layout::SETTING_VALUE_MIN_WIDTH_VW, toggle::spawn_toggle};
use crate::states::{RunningState, running::options::systems::theming::ToggleValue};

/// A setting row's caption text (the label to the left of its toggle).
///
/// A named newtype over `&'static str` (no-bare-types rule) so a row spec cannot silently
/// swap its caption and its value readout.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SettingCaption(&'static str);

impl SettingCaption {
    /// Wrap a caption string as a typed setting caption.
    pub(super) const fn new(caption: &'static str) -> Self {
        Self(caption)
    }
}

/// A setting row's value readout text ("On" / "Off"), as produced by that setting's own
/// `*_value_text` formatter.
///
/// A named newtype over `&'static str` (no-bare-types rule), paired with
/// [`SettingCaption`].
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SettingValueText(&'static str);

impl SettingValueText {
    /// Wrap a readout string as a typed setting value text.
    pub(super) const fn new(text: &'static str) -> Self {
        Self(text)
    }
}

/// Everything one setting row needs: its text, its current value, and the three marker
/// bundles that give its nodes identity (the toggle, the toggle's knob, the readout).
///
/// A single typed parameter rather than eight positional arguments (clippy
/// `too_many_arguments`, and it reads at the call site).
pub(super) struct SettingRowSpec<V, M, K, L> {
    /// The caption to the left of the toggle.
    pub(super) caption:       SettingCaption,
    /// The setting's current value, which seeds the toggle's checked state and paint.
    pub(super) value:         V,
    /// The readout text for `value`.
    pub(super) value_text:    SettingValueText,
    /// The marker bundle identifying the toggle checkbox.
    pub(super) toggle_marker: M,
    /// The marker bundle identifying the toggle's knob child.
    pub(super) knob_marker:   K,
    /// The marker bundle identifying the value readout text node.
    pub(super) value_marker:  L,
}

/// The entities a spawned setting row hands back: the row container (to parent into the
/// panel) and the toggle checkbox (to wire into the navigation chain).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SettingRow {
    /// The row container node.
    pub(super) row:    Entity,
    /// The row's toggle checkbox — a focusable control.
    pub(super) toggle: Entity,
}

/// Spawns one setting row (caption, pill toggle, value readout) and returns its entities.
///
/// Every node carries [`DespawnOnExit(RunningState::Options)`] so the row dies with the
/// screen. The readout gets a fixed `min_width` floor so flipping the toggle never reflows
/// the centered screen (GTW-800a).
pub(super) fn spawn_setting_row<V: ToggleValue, M: Bundle, K: Bundle, L: Bundle>(
    commands: &mut Commands,
    theme: &GdtfTheme,
    spec: SettingRowSpec<V, M, K, L>,
) -> SettingRow {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Vw(*super::layout::OptionsGapVh::SCREEN),
                ..default()
            },
            DespawnOnExit(RunningState::Options),
        ))
        .id();

    let caption = *spec.caption;
    let label = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Text) Text::new(caption) },
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    let toggle = spawn_toggle(
        commands,
        theme,
        spec.value,
        spec.toggle_marker,
        spec.knob_marker,
    );

    // `Text` brings its own default `Node`; this replaces it with one carrying the
    // width floor (the themed Text paint touches color/font, not layout width).
    let value_node = Node {
        min_width: Val::Vw(SETTING_VALUE_MIN_WIDTH_VW),
        ..default()
    };
    let value_text = *spec.value_text;
    let value = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Text) Text::new(value_text) },
            template_value(value_node),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();
    commands.entity(value).insert(spec.value_marker);

    commands.entity(row).add_children(&[label, toggle, value]);
    SettingRow { row, toggle }
}

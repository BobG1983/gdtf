//! Spawns the Options screen on `OnEnter(RunningState::Options)` (GTW-637).
//!
//! The screen is authored `bsn!`-first (the GTW-322 macro path) with first-party
//! widgets, the pilot that measures whether "all new screens are `bsn!`" holds. It
//! builds a themed, full-screen backdrop holding a title above a panel box that wraps
//! the setting rows and a Continue button:
//!
//! - **Root** — a full-screen [`Themed(Background)`](gdtf_ui::themed::ThemeRole)
//!   [`Node`], a centered flex column (marker [`OptionsScreenRoot`]).
//! - **Title** — a [`Themed(Title)`](gdtf_ui::themed::ThemeRole) "OPTIONS" heading,
//!   a direct child of the root (marker [`OptionsTitle`]).
//! - **Panel box** — [`spawn_panel`](gdtf_ui::spawn_panel), a flex column, wrapping
//!   the setting rows and the Continue button.
//! - **Sound row** — a [`spawn_setting_row`] row: a "Sound" caption, the first-party
//!   [`bevy_ui_widgets::Checkbox`](bevy::ui_widgets::Checkbox) marked [`SoundToggle`]
//!   rendered as a themed pill toggle, and a value readout (marker [`SoundValueLabel`]).
//! - **Procgen-stepper row** — the SAME row shape, present ONLY in a `dev_tools` build
//!   (GTW-868); see `super::stepper_row`. A non-`dev_tools` build shows the screen exactly
//!   as it did before that ticket.
//! - **Continue button** — [`spawn_button`](gdtf_ui::spawn_button) captioned
//!   "Continue" (marker [`ContinueButton`]); activating it returns to the Main Menu
//!   ([`RunningState::Menu`](crate::states::RunningState::Menu)) — the screen Options
//!   was opened from (GTW-801).
//!
//! Every entity carries [`DespawnOnExit(RunningState::Options)`] so the whole tree is
//! torn down on leave. Every focusable control — each setting toggle and the Continue
//! button — is joined into ONE vertical [`DirectionalNavigationMap`] chain in spawn order,
//! so the player can move focus between them with the arrow keys / gamepad D-pad and the
//! generic focus-activation path reaches each of them identically. The first toggle grabs
//! initial focus so `Enter` / `Space` activates the checkbox's native keyboard path
//! immediately.

use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap,
    math::CompassOctant,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_ui::{
    ButtonLabel,
    focus_nav::set_initial_focus,
    spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{
    layout::OptionsGapVh,
    row::{SettingCaption, SettingRowSpec, SettingValueText, spawn_setting_row},
};
use crate::states::{
    RunningState,
    running::options::{
        components::{
            ContinueButton, OptionsScreenRoot, OptionsTitle, SoundToggle, SoundToggleKnob,
            SoundValueLabel,
        },
        settings::{GameSettings, sound_value_text},
    },
};

/// Builds the full Options screen when [`RunningState::Options`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (bevy-traps rule 1) — in the running app the theme is present by the time the player
/// can reach Options. It also reads [`GameSettings`] to seed each toggle's initial on/off
/// state and its value readout, so re-entering the screen shows the persisted settings.
/// See the module docs for the tree it builds.
pub(in crate::states::running::options) fn spawn_options_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    settings: Res<GameSettings>,
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    let Some(theme) = theme else {
        return;
    };

    // ROOT: the centered, full-screen backdrop (Themed(Background)); the layout
    // rides `template_value` (a runtime-valued `Node` has no bsn! value grammar).
    let root_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Vh(*OptionsGapVh::SCREEN),
        ..default()
    };
    let root = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Background) OptionsScreenRoot },
            template_value(root_node),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    // TITLE — a Themed(Title) heading, a direct child of the root.
    let title = commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("OPTIONS")
                OptionsTitle
            },
            template_value(TextLayout::justify(Justify::Center)),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    // PANEL BOX — a themed panel laid out as a column that stretches its children.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::Options),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(*OptionsGapVh::SCREEN),
            ..default()
        },
    ));

    // SOUND ROW — the ONE player-facing setting: caption, first-party Checkbox pill,
    // readout. Tagged `SoundToggle` so the widget's native `ValueChange<bool>` observer
    // maps its activation to the typed `SoundSettingChanged` intent.
    let sound = settings.sound;
    let sound_row = spawn_setting_row(
        &mut commands,
        &theme,
        SettingRowSpec {
            caption:       SettingCaption::new("Sound"),
            value:         sound,
            value_text:    SettingValueText::new(sound_value_text(sound)),
            toggle_marker: SoundToggle,
            knob_marker:   SoundToggleKnob,
            value_marker:  SoundValueLabel,
        },
    );

    // The panel's children and the focus chain, in the same order: every setting row's
    // toggle, then Continue. The dev-only stepper row (if compiled in) slots between them.
    let mut rows = vec![sound_row.row];
    let mut focusables = vec![sound_row.toggle];
    #[cfg(feature = "dev_tools")]
    {
        let stepper_row = super::stepper_row::spawn_stepper_row(&mut commands, &theme, *settings);
        rows.push(stepper_row.row);
        focusables.push(stepper_row.toggle);
    }

    // CONTINUE — a themed button that returns to the Main Menu.
    let continue_button = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Continue"),
        (ContinueButton, DespawnOnExit(RunningState::Options)),
    );
    commands.entity(continue_button).insert(Node {
        width: Val::Percent(100.0),
        ..default()
    });
    focusables.push(continue_button);
    rows.push(continue_button);

    // Parent the tree: root → [title, panel]; panel → [setting rows…, continue].
    commands.entity(root).add_children(&[title, panel]);
    commands.entity(panel).add_children(&rows);

    // A non-wrapping vertical nav chain over every focusable control, so the arrow keys /
    // D-pad move focus between them. `add_edges` lays the forward South edges and the
    // reverse North edges (it needs the whole chain). The dev-only stepper toggle is just
    // another link — no special case, so the generic focus-activation path drives it.
    nav_map.add_edges(&focusables, CompassOctant::South);

    // The first toggle grabs initial focus so `Enter` / `Space` fires the checkbox's
    // native keyboard activation the moment the screen opens.
    set_initial_focus(&mut commands, sound_row.toggle);
}

/// Clears the Options screen's entries from the global [`DirectionalNavigationMap`] on
/// `OnExit(RunningState::Options)`.
///
/// The screen's focusable entities are despawned by their
/// [`DespawnOnExit(RunningState::Options)`] markers, but the
/// [`DirectionalNavigationMap`] is a GLOBAL resource (not state-scoped), so the edges
/// keyed by those now-dead entity ids would linger as stale neighbors and poison the
/// next screen's chain (the menu / battlescape both `add_edges` WITHOUT clearing
/// first). Clearing wholesale on exit mirrors the menu's own `clear_nav_map` and the
/// battlescape's `clear_panel_nav_topology`, and is ordering-independent (a wholesale
/// `clear`, not a per-entity `remove` that would need the entities still alive).
pub(in crate::states::running::options) fn clear_options_nav_map(
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    nav_map.clear();
}

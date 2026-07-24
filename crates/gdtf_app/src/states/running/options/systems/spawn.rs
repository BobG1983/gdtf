//! Spawns the Options screen on `OnEnter(RunningState::Options)` (GTW-637).
//!
//! The screen is authored `bsn!`-first (the GTW-322 macro path) with first-party
//! widgets, the pilot that measures whether "all new screens are `bsn!`" holds. It
//! builds a themed, full-screen backdrop holding a title above a panel box that wraps
//! one setting row and a Continue button:
//!
//! - **Root** — a full-screen [`Themed(Background)`](gdtf_ui::themed::ThemeRole)
//!   [`Node`], a centered flex column (marker [`OptionsScreenRoot`]).
//! - **Title** — a [`Themed(Title)`](gdtf_ui::themed::ThemeRole) "OPTIONS" heading,
//!   a direct child of the root (marker [`OptionsTitle`]).
//! - **Panel box** — [`spawn_panel`](gdtf_ui::spawn_panel), a flex column, wrapping
//!   the setting row and the Continue button.
//! - **Sound row** — a "Sound" label ([`Themed(Text)`](gdtf_ui::themed::ThemeRole)),
//!   the ONE first-party widget — a [`bevy_ui_widgets::Checkbox`](bevy::ui_widgets::Checkbox)
//!   marked [`SoundToggle`], rendered as a themed pill toggle whose visual the screen
//!   builds (the widget is headless) — and a value readout
//!   ([`Themed(Text)`](gdtf_ui::themed::ThemeRole), marker [`SoundValueLabel`]).
//! - **Continue button** — [`spawn_button`](gdtf_ui::spawn_button) captioned
//!   "Continue" (marker [`ContinueButton`]); activating it returns to the Main Menu
//!   ([`RunningState::Menu`](crate::states::RunningState::Menu)) — the screen Options
//!   was opened from (GTW-801).
//!
//! Every entity carries [`DespawnOnExit(RunningState::Options)`] so the whole tree is
//! torn down on leave. The sound toggle and the Continue button are joined by a
//! vertical [`DirectionalNavigationMap`] edge so the player can move focus between them
//! with the arrow keys / gamepad D-pad; the toggle grabs initial focus so `Enter` /
//! `Space` activates the checkbox's native keyboard path immediately.

use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap,
    math::CompassOctant,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::{BorderColor as UiBorderColor, BorderRadius, Checked, UiRect, Val},
    ui_widgets::Checkbox,
};
use gdtf_ui::{
    ButtonLabel,
    focus_nav::set_initial_focus,
    spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::options::{
        components::{
            ContinueButton, OptionsScreenRoot, OptionsTitle, SoundToggle, SoundToggleKnob,
            SoundValueLabel,
        },
        settings::{GameSettings, SoundEnabled, sound_value_text},
        systems::theming::{knob_justify, sound_toggle_colors},
    },
};

/// The Options column's inter-child gap, in viewport-height percent (`Vh`).
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it
/// is layout spacing set on the flex column [`Node`] directly (the theme owns
/// palette + font, not inter-child layout), mirroring the menu's `ColumnGapVh`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct OptionsGapVh(f32);

impl OptionsGapVh {
    /// The screen's column gap, matching the menu's 10 px / 720 reference gap.
    const SCREEN: Self = Self(1.38889);
}

/// The sound toggle pill's long (slide) dimension, in viewport-width units.
/// Calibrated to the hand-rolled switch's 48 px / 1280 pill so the toggle keeps the
/// same footprint beside its "Sound" caption.
const TOGGLE_LONG_VW: f32 = 3.75;

/// The sound toggle pill's short dimension, in viewport-width units (20 px / 1280) —
/// strictly taller than the knob so the pill frames it.
const TOGGLE_SHORT_VW: f32 = 1.562_5;

/// The inner padding around the knob inside the toggle pill, in viewport-width units
/// (3 px / 1280) — keeps the knob clear of the rounded caps.
const TOGGLE_PAD_VW: f32 = 0.234_375;

/// The knob diameter, in viewport-width units (14 px / 1280), smaller than the pill's
/// short dimension so the track frames it.
const KNOB_DIAMETER_VW: f32 = 1.093_75;

/// The sound value readout's fixed minimum width, in viewport-width units (32 px /
/// 1280) — sized to comfortably clear the WIDER of the two readout strings ("Off",
/// 3 chars at the 18 pt body font vs "On"'s 2). Fixing a floor on the label's width
/// means flipping the toggle never changes the label's intrinsic width, so the
/// auto-width sound row / panel / centered screen no longer reflows on every toggle
/// (GTW-800a).
const SOUND_VALUE_MIN_WIDTH_VW: f32 = 2.5;

/// Builds the full Options screen when [`RunningState::Options`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is
/// absent (bevy-traps rule 1) — in the running app the theme is present by the time
/// the player can reach Options. It also reads [`GameSettings`] to seed the sound
/// toggle's initial on/off state and its value readout, so re-entering the screen
/// shows the persisted setting. See the module docs for the tree it builds.
pub(in crate::states::running::options) fn spawn_options_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    settings: Res<GameSettings>,
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    let Some(theme) = theme else {
        return;
    };

    let sound = settings.sound;

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

    // SOUND ROW — a bare layout row (no theme role of its own) holding the label,
    // the toggle widget, and the value readout.
    let sound_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Vw(*OptionsGapVh::SCREEN),
                ..default()
            },
            DespawnOnExit(RunningState::Options),
        ))
        .id();

    let sound_label = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Text) Text::new("Sound") },
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    // The ONE first-party widget: a real `bevy_ui_widgets::Checkbox`, rendered as a
    // themed pill toggle (the widget is headless — it ships no visual). It is seeded
    // from the persisted setting (the `Checked` component IS the checkbox's state,
    // kept in step by the first-party `checkbox_self_update` observer the plugin
    // registers) and tagged `SoundToggle` so the widget's native `ValueChange<bool>`
    // observer maps its activation to the typed `SoundSettingChanged` intent.
    let sound_toggle = spawn_sound_toggle(&mut commands, &theme, sound);

    // A fixed min-width floor on the readout so "On" (2 chars) and "Off" (3 chars)
    // occupy the same width — the toggle flip no longer reflows the centered screen
    // (GTW-800a). `Text` brings its own default `Node`; this replaces it with one
    // carrying the floor (the themed Text paint touches color/font, not layout width).
    let sound_value_node = Node {
        min_width: Val::Vw(SOUND_VALUE_MIN_WIDTH_VW),
        ..default()
    };
    let sound_value = commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Text)
                Text::new(sound_value_text(sound))
                SoundValueLabel
            },
            template_value(sound_value_node),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    // CONTINUE — a themed button that leaves the screen for `Game`.
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

    // Parent the tree: root → [title, panel]; panel → [sound_row, continue];
    // sound_row → [label, toggle, value].
    commands.entity(root).add_children(&[title, panel]);
    commands
        .entity(panel)
        .add_children(&[sound_row, continue_button]);
    commands
        .entity(sound_row)
        .add_children(&[sound_label, sound_toggle, sound_value]);

    // A non-wrapping vertical nav chain over the two focusable controls, so the arrow
    // keys / D-pad move focus between the sound toggle and Continue. `add_edges` lays
    // the forward South edges and the reverse North edges (it needs both nodes).
    nav_map.add_edges(&[sound_toggle, continue_button], CompassOctant::South);

    // The sound toggle grabs initial focus so `Enter` / `Space` fires the checkbox's
    // native keyboard activation the moment the screen opens; Continue is one nav step
    // away.
    set_initial_focus(&mut commands, sound_toggle);
}

/// Spawns the sound-toggle [`Checkbox`](bevy::ui_widgets::Checkbox) as a themed pill
/// toggle and returns the checkbox [`Entity`].
///
/// The checkbox is headless, so this builds its whole visual: a rounded track [`Node`]
/// (colored from the theme for the current on/off state, with an opaque themed pill
/// outline so it stays visible against the panel when OFF — GTW-800b) with one knob
/// child (marker [`SoundToggleKnob`], justified to the on/off end). It seeds the
/// first-party
/// [`Checked`](bevy::ui::Checked) state when sound is on, tags the root [`SoundToggle`]
/// so the native `ValueChange` observer + the theming pass find it, and marks the
/// whole tree [`DespawnOnExit(RunningState::Options)`].
fn spawn_sound_toggle(commands: &mut Commands, theme: &GdtfTheme, sound: SoundEnabled) -> Entity {
    let colors = sound_toggle_colors(theme);
    let track_node = Node {
        width: Val::Vw(TOGGLE_LONG_VW),
        height: Val::Vw(TOGGLE_SHORT_VW),
        flex_direction: FlexDirection::Row,
        padding: UiRect::all(Val::Vw(TOGGLE_PAD_VW)),
        align_items: AlignItems::Center,
        justify_content: knob_justify(sound),
        // A visible pill outline (from the button sub-theme's border, opaque) so the
        // track reads against the panel in BOTH states — the OFF fill alone
        // (theme.button.disabled, near-black at 0.55 alpha) blends into the
        // semi-transparent panel and the pill was invisible when off (GTW-800b).
        border: UiRect::all(Val::Vw(*theme.button.border_width)),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let knob_node = Node {
        width: Val::Vw(KNOB_DIAMETER_VW),
        height: Val::Vw(KNOB_DIAMETER_VW),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let knob = commands
        .spawn((
            SoundToggleKnob,
            BackgroundColor(colors.knob()),
            knob_node,
            DespawnOnExit(RunningState::Options),
        ))
        .id();
    let toggle = commands
        .spawn((
            Checkbox,
            SoundToggle,
            BackgroundColor(colors.track(sound)),
            UiBorderColor::all(colors.border()),
            track_node,
            DespawnOnExit(RunningState::Options),
        ))
        .id();
    if sound.is_on() {
        commands.entity(toggle).insert(Checked);
    }
    commands.entity(toggle).add_children(&[knob]);
    toggle
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

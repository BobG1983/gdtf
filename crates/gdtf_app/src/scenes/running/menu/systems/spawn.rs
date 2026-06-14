//! Spawns the main-menu scene on `OnEnter(RunningState::Menu)` (GTW-121).
//!
//! Builds the Godot-faithful menu: a full-screen, centered vertical column
//! (built via [`gdtf_ui::spawn_panel`]) holding, top-to-bottom, the title, then
//! the Battlescape / Options / `HiveScape` (disabled) / Quit buttons (each built
//! via [`gdtf_ui::spawn_button`]). Every entity carries the
//! [`Themed`](gdtf_ui::themed::Themed) marker (so GTW-137's live re-theme
//! restyles them) and [`DespawnOnExit(RunningState::Menu)`](bevy::prelude::DespawnOnExit)
//! (so the whole tree is torn down on leave). Battlescape grabs initial focus,
//! and the enabled buttons are wired into a non-wrapping vertical navigation
//! chain.
//!
//! Button *actions* (what happens when a button is activated) are GTW-122; this
//! system only constructs the scene, its markers, the initial focus, and the
//! nav graph.

use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap, math::CompassOctant, prelude::*,
    ui::Val,
};
use gdtf_ui::{
    ButtonLabel, DisabledButton,
    focus_nav::set_initial_focus,
    spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::{
    scenes::running::menu::components::{
        BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    },
    states::RunningState,
};

/// Vertical gap between the menu column's children, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule):
/// it mirrors the Godot `VBoxContainer` `separation = 10`. It is layout spacing,
/// not a theme color/size, so it is set on the column [`Node`] directly (the
/// theme owns palette + font, not inter-child layout).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ColumnGapPx(f32);

impl ColumnGapPx {
    /// The Godot menu's `VBoxContainer` separation: 10 px between children.
    const MENU: Self = Self(10.0);
}

/// Builds the full main-menu scene when [`RunningState::Menu`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is
/// absent (bevy-traps rule 1) — in the running app the theme is present
/// post-GTW-143. With the theme present it:
///
/// 1. Spawns the centered, full-screen flex column via [`spawn_panel`] and lays
///    out its layout fields (the theme owns palette/border/radius/padding;
///    `apply_theme` preserves these layout fields on re-run).
/// 2. Spawns the title and the four buttons via [`spawn_button`], each tagged
///    [`Themed`], [`DespawnOnExit(RunningState::Menu)`](DespawnOnExit), and its
///    role marker; `HiveScape` additionally carries [`DisabledButton`].
/// 3. Orders them under the column top→bottom: Title, Battlescape, Options,
///    `HiveScape`, Quit (`HiveScape` directly above Quit).
/// 4. Sets initial focus to Battlescape via [`set_initial_focus`].
/// 5. Wires the **enabled** buttons (Battlescape ↔ Options ↔ Quit) into a
///    non-wrapping vertical nav chain via
///    [`DirectionalNavigationMap::add_edges`]; the disabled `HiveScape` is omitted
///    from the chain (0.18.1 has no built-in skip).
pub(in crate::scenes::running::menu) fn spawn_menu(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    let Some(theme) = theme else {
        // No theme yet (pre-Load) — nothing to style from; spawn nothing rather
        // than paint an un-themed menu. The running app always has it by Menu.
        return;
    };

    // The centered, full-screen flex column. `spawn_panel` paints the theme look;
    // we add the layout (full size, column, centered, inter-child gap). These
    // layout fields survive `apply_theme`, which only overrides the theme-owned
    // border / radius / padding while cloning the rest of the node.
    let column = spawn_panel(&mut commands, &theme);
    commands.entity(column).insert((
        DespawnOnExit(RunningState::Menu),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(*ColumnGapPx::MENU),
            ..default()
        },
    ));

    // Title — a larger, theme-derived heading (ThemeRole::Title), not a button.
    let title = commands
        .spawn((
            Themed(ThemeRole::Title),
            Text::new("GRIMDARK TURFWAR"),
            TextLayout::new_with_justify(Justify::Center),
            MenuTitle,
            DespawnOnExit(RunningState::Menu),
        ))
        .id();

    // The four buttons, each themed + state-scoped via the per-button marker
    // bundle passed to `spawn_button`.
    let battlescape = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Battlescape"),
        (BattlescapeButton, DespawnOnExit(RunningState::Menu)),
    );
    let options = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Options"),
        (OptionsButton, DespawnOnExit(RunningState::Menu)),
    );
    // HiveScape: disabled placeholder for the campaign layer, directly above Quit.
    let hivescape = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("HiveScape"),
        (
            HiveScapeButton,
            DisabledButton,
            DespawnOnExit(RunningState::Menu),
        ),
    );
    let quit = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Quit"),
        (QuitButton, DespawnOnExit(RunningState::Menu)),
    );

    // Parent everything under the column in visual order, top→bottom.
    commands
        .entity(column)
        .add_children(&[title, battlescape, options, hivescape, quit]);

    // Battlescape grabs initial focus (Godot `%BattlescapeButton.grab_focus()`).
    set_initial_focus(&mut commands, battlescape);

    // Non-wrapping vertical nav chain over the ENABLED buttons only. `add_edges`
    // wires symmetrical North/South edges between consecutive entries and does
    // NOT loop, matching the Godot menu's non-wrapping focus. The disabled
    // HiveScape is omitted (0.18.1 has no built-in skip-disabled).
    nav_map.add_edges(&[battlescape, options, quit], CompassOctant::South);
}

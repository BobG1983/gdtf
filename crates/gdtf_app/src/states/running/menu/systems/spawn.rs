//! Spawns the main-menu scene on `OnEnter(RunningState::Menu)` (GTW-121, nested
//! theme rewire GTW-149).
//!
//! Builds the menu tree as a full-screen, centered backdrop holding the title
//! floating above a panel box that wraps the four buttons:
//!
//! - **Root** — a full-screen [`Themed(Background)`](gdtf_ui::themed::ThemeRole)
//!   [`Node`](bevy::ui::Node) (the backdrop fill), a centered flex column.
//! - **Title** — a [`Themed(Title)`](gdtf_ui::themed::ThemeRole) text, a *direct*
//!   child of the root, so it floats on the backdrop (NOT inside the panel).
//! - **Panel box** — [`gdtf_ui::spawn_panel`] ([`Themed(Panel)`](gdtf_ui::themed::ThemeRole)),
//!   a flex column with `align_items: Stretch` so its button children are equal
//!   width; it wraps ONLY the four buttons.
//! - **Buttons** — Battlescape / Options / `HiveScape` (disabled) / Quit, each
//!   built via [`gdtf_ui::spawn_button`] (a [`Themed(Button)`](gdtf_ui::themed::ThemeRole)
//!   box with a [`Themed(ButtonText)`](gdtf_ui::themed::ThemeRole) caption child),
//!   children of the panel box.
//!
//! Every entity carries the [`Themed`](gdtf_ui::themed::Themed) marker (so the
//! live re-theme restyles them) and
//! [`DespawnOnExit(RunningState::Menu)`](bevy::prelude::DespawnOnExit) (so the
//! whole tree is torn down on leave). Battlescape grabs initial focus, and the
//! enabled buttons are wired into a non-wrapping vertical navigation chain.
//!
//! Button *actions* (what happens when a button is activated) are GTW-122; this
//! system only constructs the scene, its markers, the initial focus, and the
//! nav graph.

use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap,
    math::CompassOctant,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_ui::{
    ButtonLabel, DisabledButton,
    focus_nav::set_initial_focus,
    spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::menu::components::{
        BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    },
};

/// Vertical gap between the menu column's children, as a viewport-height
/// percentage (`Vh`).
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule):
/// it mirrors the Godot `VBoxContainer` `separation = 10` px, calibrated to the
/// 1280×720 reference window as `10 / 720 * 100 = 1.38889` Vh so the rendered
/// gap is pixel-identical at the default size while scaling with the window
/// (GTW-296 px→relative sweep). It is layout spacing, not a theme color/size, so
/// it is set on the column [`Node`] directly (the theme owns palette + font, not
/// inter-child layout).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ColumnGapVh(f32);

impl ColumnGapVh {
    /// The Godot menu's `VBoxContainer` separation (10 px at 1280×720),
    /// expressed in `Vh`: `10 / 720 * 100 = 1.38889`.
    const MENU: Self = Self(1.38889);
}

/// Builds the full main-menu scene when [`RunningState::Menu`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is
/// absent (bevy-traps rule 1) — in the running app the theme is present by `Menu`.
/// With the theme present it:
///
/// 1. Spawns the centered, full-screen backdrop root: a
///    [`Themed(Background)`](ThemeRole::Background) [`Node`] (the backdrop fill),
///    a centered flex column with an inter-child gap.
/// 2. Spawns the title as a *direct* child of the root — a
///    [`Themed(Title)`](ThemeRole::Title) heading floating on the backdrop, not
///    inside the panel.
/// 3. Spawns the panel box via [`spawn_panel`] (a [`Themed(Panel)`](ThemeRole::Panel)
///    box), a child of the root, laid out as a flex column with
///    `align_items: Stretch` so its button children are equal width; it wraps
///    only the four buttons.
/// 4. Spawns the four buttons via [`spawn_button`] (each
///    [`Themed(Button)`](ThemeRole::Button) with a
///    [`Themed(ButtonText)`](ThemeRole::ButtonText) caption child), children of
///    the panel box in order Battlescape, Options, `HiveScape`, Quit; `HiveScape`
///    additionally carries [`DisabledButton`]. Each button is full width so the
///    panel's `Stretch` makes them equal width.
/// 5. Sets initial focus to Battlescape via [`set_initial_focus`].
/// 6. Wires the **enabled** buttons (Battlescape ↔ Options ↔ Quit) into a
///    non-wrapping vertical nav chain via
///    [`DirectionalNavigationMap::add_edges`]; the disabled `HiveScape` is omitted
///    from the chain (0.18.1 has no built-in skip).
pub(in crate::states::running::menu) fn spawn_menu(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    let Some(theme) = theme else {
        // No theme yet (pre-Load) — nothing to style from; spawn nothing rather
        // than paint an un-themed menu. The running app always has it by Menu.
        return;
    };

    // ROOT: the centered, full-screen backdrop. `Themed(Background)` paints the
    // backdrop fill; the layout (full size, centered column, inter-child gap)
    // survives `apply_theme`, which writes only BackgroundColor for this role.
    //
    // GTW-322 — authored as a `bsn!` scene. `Themed` rides the macro inline (its
    // `Themed::new` ctor + sentinel `Default`); the runtime-valued `Node` (the
    // layout) and `DespawnOnExit` (state-scope) have no `bsn!` value grammar, so
    // they are composed onto the same entity with `template_value`.
    let root_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Vh(*ColumnGapVh::MENU),
        ..default()
    };
    let root = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Background) },
            template_value(root_node),
            template_value(DespawnOnExit(RunningState::Menu)),
        ))
        .id();

    // TITLE — a theme-derived heading (ThemeRole::Title), a DIRECT child of the
    // root so it floats on the backdrop above the panel.
    //
    // GTW-322 — `Themed`, `Text::new`, and the `MenuTitle` marker ride the `bsn!`
    // macro inline; the runtime-valued `TextLayout` (justify) and `DespawnOnExit`
    // are composed with `template_value`.
    let title = commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("GRIMDARK TURFWAR")
                MenuTitle
            },
            template_value(TextLayout::justify(Justify::Center)),
            template_value(DespawnOnExit(RunningState::Menu)),
        ))
        .id();

    // PANEL BOX — `spawn_panel` paints the panel look; we add a column layout with
    // `align_items: Stretch` so the button children are equal width = the panel's
    // content width, and a gap. It is sized to content (no width/height set), and
    // wraps ONLY the four buttons. The layout fields survive `apply_theme` (it
    // overrides only the theme-owned border / radius / padding for the Panel role).
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::Menu),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(*ColumnGapVh::MENU),
            ..default()
        },
    ));

    // The four buttons, each themed + state-scoped via the per-button marker
    // bundle passed to `spawn_button`. Width 100% so the panel's Stretch makes
    // them equal width (= the panel content width).
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

    // Each button stretches to the panel content width (equal-width buttons).
    for button in [battlescape, options, hivescape, quit] {
        commands.entity(button).insert(Node {
            width: Val::Percent(100.0),
            ..default()
        });
    }

    // Order under root: [title, panel-box]; order under panel-box: the four
    // buttons top→bottom (Battlescape, Options, HiveScape, Quit).
    commands.entity(root).add_children(&[title, panel]);
    commands
        .entity(panel)
        .add_children(&[battlescape, options, hivescape, quit]);

    // Battlescape grabs initial focus (Godot `%BattlescapeButton.grab_focus()`).
    set_initial_focus(&mut commands, battlescape);

    // Non-wrapping vertical nav chain over the ENABLED buttons only. `add_edges`
    // wires symmetrical North/South edges between consecutive entries and does
    // NOT loop, matching the Godot menu's non-wrapping focus. The disabled
    // HiveScape is omitted (0.18.1 has no built-in skip-disabled).
    nav_map.add_edges(&[battlescape, options, quit], CompassOctant::South);
}

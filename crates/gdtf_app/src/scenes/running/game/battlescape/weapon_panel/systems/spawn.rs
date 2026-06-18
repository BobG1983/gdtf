//! Spawns + despawns the battlescape weapon panel (GTW-275, bottom-left).
//!
//! [`spawn_weapon_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] panel on the GTW-120 UI camera: a [`spawn_panel`] box
//! ([`WeaponPanelRoot`]) anchored BOTTOM-LEFT, holding a weapon-content column (a
//! graphic PLACEHOLDER box, the weapon name, the magazine `"cur/max"` text, a LIVE
//! [`ReloadButton`]) and a throwable placeholder row. The content starts at the empty
//! state ([`Visibility::Hidden`]); [`update_weapon_panel`](super::update::update_weapon_panel)
//! reveals + repaints it from the selection every battle frame by mutating the existing
//! widgets ([[ui-mutate-not-respawn]]).
//!
//! [`despawn_weapon_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole panel by its [`WeaponPanelRoot`] marker — battle-scoped, mirroring
//! the sibling status panel / action bar.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{Node, UiRect, Val},
};
use gdtf_ui::{
    ButtonLabel, spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::scenes::running::game::battlescape::weapon_panel::components::{
    ReloadButton, WeaponContent, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
};

/// The side length of a placeholder graphic box, in logical pixels — sized like the
/// status panel's portrait (`PORTRAIT_PX = 56.0`) so the weapon graphic + throwable
/// slots read at a comparable scale (AC5 / AC7).
///
/// A `const`, layout plumbing fed straight to a [`Node`] (the `CELL_PX`-class carve-out,
/// not a domain value).
const PLACEHOLDER_PX: f32 = 56.0;

/// The vertical gap between weapon-panel rows, in logical pixels (the stat-block
/// `ROW_GAP_PX` precedent). A `const`, layout plumbing fed to a [`Node`].
const ROW_GAP_PX: f32 = 4.0;

/// Builds a PLACEHOLDER graphic box — a themed-panel-role square [`Node`] with a
/// "no image available" caption, standing in for per-weapon / throwable art that does
/// NOT exist (the items atlas is deliberately not loaded, AC5 / AC7).
///
/// Returns the box [`Entity`]; the caller parents it. It is a `Themed(Panel)` square so
/// it reads as an empty slot in the panel's look (re-painted by `apply_theme` like any
/// themed node), carrying a small centered caption.
fn spawn_placeholder(commands: &mut Commands, theme: &GdtfTheme, caption: &str) -> Entity {
    let slot = commands
        .spawn((
            Themed(ThemeRole::Panel),
            Node {
                width: Val::Px(PLACEHOLDER_PX),
                height: Val::Px(PLACEHOLDER_PX),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(*theme.panel.border_width_px)),
                ..default()
            },
            BackgroundColor(*theme.panel.color),
            bevy::ui::BorderColor::all(*theme.panel.border_color),
        ))
        .id();
    let label = commands
        .spawn((
            Themed(ThemeRole::Text),
            Text::new(caption),
            TextFont {
                font: theme.text.font.clone(),
                font_size: *theme.text.font_size_pt,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
        ))
        .id();
    commands.entity(slot).add_children(&[label]);
    slot
}

/// Spawns a themed [`Text`] line tagged `marker`, started empty — the name / magazine
/// lines (the stat-block `spawn_text` precedent). The update mutates the text in place.
fn spawn_text(commands: &mut Commands, theme: &GdtfTheme, marker: impl Bundle) -> Entity {
    commands
        .spawn((
            marker,
            Themed(ThemeRole::Text),
            Text::new(""),
            TextFont {
                font: theme.text.font.clone(),
                font_size: *theme.text.font_size_pt,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
        ))
        .id()
}

/// Builds the themed weapon panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle
/// starts; the status-panel precedent). With the theme present it spawns the
/// [`WeaponPanelRoot`] panel box anchored BOTTOM-LEFT (the free corner, AC5 / AC9) and
/// fills it with:
///
/// 1. a [`WeaponContent`] column (started [`Visibility::Hidden`] — the AC9 empty state):
///    a graphic PLACEHOLDER box, the [`WeaponNameText`] line, the [`WeaponMagazineText`]
///    `"cur/max"` line, and the LIVE [`ReloadButton`];
/// 2. a throwable placeholder row (AC7 — `Visibility::Hidden` slots that are not modeled).
///
/// The content widgets start empty / hidden; [`update_weapon_panel`](super::update::update_weapon_panel)
/// reveals + repaints them from the selection by MUTATING the existing widgets
/// ([[ui-mutate-not-respawn]]). Param-only (`bevy-traps.md` #7): [`Commands`] + the theme read.
pub(in crate::scenes::running::game::battlescape) fn spawn_weapon_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the status-panel
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // ROOT: the bottom-left anchored panel box (the FREE corner, AC5 / AC9). `spawn_panel`
    // paints the panel look; the layout fields survive `apply_theme` (it overrides only the
    // theme-owned border / radius / padding for the Panel role — the status-panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        WeaponPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(ROW_GAP_PX),
            ..default()
        },
    ));

    // 1. The weapon-content column — hidden until a weapon is selected (AC9 empty state).
    let graphic = spawn_placeholder(&mut commands, &theme, "no image");
    let name = spawn_text(&mut commands, &theme, WeaponNameText);
    let magazine = spawn_text(&mut commands, &theme, WeaponMagazineText);
    let reload = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Reload"),
        ReloadButton,
    );
    let content = commands
        .spawn((
            WeaponContent,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(ROW_GAP_PX),
                ..default()
            },
            // Hidden until a weapon is selected (AC9). The update reveals it.
            Visibility::Hidden,
        ))
        .id();
    commands
        .entity(content)
        .add_children(&[graphic, name, magazine, reload]);

    // 2. The throwable placeholder row (AC7) — slots for an unmodeled feature, deliberate
    //    deferred placeholders sized like the weapon graphic. They are inert boxes (no
    //    marker, no interaction) and always shown as empty slots.
    let throwable_a = spawn_placeholder(&mut commands, &theme, "throw");
    let throwable_b = spawn_placeholder(&mut commands, &theme, "throw");
    let throwables = commands
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(ROW_GAP_PX),
            ..default()
        },))
        .id();
    commands
        .entity(throwables)
        .add_children(&[throwable_a, throwable_b]);

    commands.entity(root).add_children(&[content, throwables]);
}

/// Despawns the weapon panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`WeaponPanelRoot`] entity (and so its children) so the panel
/// is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle. Param-only
/// (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<WeaponPanelRoot>>`.
pub(in crate::scenes::running::game::battlescape) fn despawn_weapon_panel(
    mut commands: Commands,
    panels: Query<Entity, With<WeaponPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}

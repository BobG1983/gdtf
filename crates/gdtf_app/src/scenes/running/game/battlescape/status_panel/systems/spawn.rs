//! Spawns + despawns the battlescape status HUD panel (GTW-252).
//!
//! [`spawn_status_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds
//! a themed [`gdtf_ui`] panel on the GTW-120 UI camera: a [`spawn_panel`] box
//! ([`StatusPanelRoot`]) anchored TOP-LEFT, holding one themed body-text
//! [`Text`](bevy::prelude::Text) child per vitals line (identity, stance, TU, HP,
//! life-state, and the GTW-254 weapon name), each carrying its per-line ZST marker so
//! [`update_status_panel`] can target that line's `Text`. The text starts at the
//! no-selection empty state and is repainted by the update system every battle frame.
//!
//! [`despawn_status_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and
//! recursively despawns the whole panel by its [`StatusPanelRoot`] marker, so the
//! panel is battle-scoped: present only during the live tactical layer, gone the
//! moment the battle leaves `BattleRunning`. The battlescape neighborhood uses
//! explicit `OnExit` cleanup (not `DespawnOnExit` state-scoping), so this mirrors the
//! sibling action-bar.
//!
//! The panel renders on the UI camera for free: a `bevy_ui` [`Node`] tree with NO
//! [`RenderLayers`](bevy::camera::visibility::RenderLayers) is routed by `bevy_ui` to
//! the highest-order camera = the GTW-120 UI camera (`bevy-traps.md` #6).

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_ui::{
    spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::scenes::running::game::battlescape::status_panel::{
    components::{
        HpText, IdentityText, LifeText, StanceText, StatusPanelRoot, TuText, WeaponNameText,
    },
    systems::labels::NO_SELECTION,
};

/// Vertical gap between the panel's vitals lines, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it is
/// layout spacing, not a theme color/size, so it is set on the panel [`Node`]
/// directly (the theme owns palette + font, not inter-child layout — the action-bar
/// `BarGapPx` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct LineGapPx(f32);

impl LineGapPx {
    /// The panel's inter-line gap: 4 px (a tight stacked vitals readout).
    const PANEL: Self = Self(4.0);
}

/// Builds the themed status panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is
/// absent (`bevy-traps.md` #1) — in the running app the theme is present by the time
/// a battle starts (it loads in `Load`). With the theme present it:
///
/// 1. Spawns the panel root via [`spawn_panel`] (a `Themed(Panel)` box), tagged
///    [`StatusPanelRoot`], laid out as a TOP-LEFT absolute vertical column with an
///    inter-line gap. The layout fields survive `apply_theme` (it overrides only the
///    theme-owned border / radius / padding for the Panel role — the action-bar panel
///    precedent).
/// 2. Spawns one body-text [`Text`](bevy::prelude::Text) child per vitals line, each
///    with its per-line ZST marker and a `Themed(ThemeRole::Text)` marker so
///    `apply_theme` restyles them on a theme change. The text starts at the
///    [`NO_SELECTION`] empty state; [`update_status_panel`](super::update::update_status_panel)
///    repaints it each battle frame from the selected ganger.
/// 3. Parents every line under the panel root.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawns + the theme read.
pub(in crate::scenes::running::game::battlescape) fn spawn_status_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel. The running app
        // always has it by the time a battle starts (the action-bar-spawn precedent).
        return;
    };

    // ROOT: the top-left anchored panel box. `spawn_panel` paints the panel look; we add a
    // vertical column layout anchored top-left via absolute positioning, plus the inter-line
    // gap. The layout fields survive `apply_theme` (it overrides only the theme-owned border
    // / radius / padding for the Panel role — the action-bar panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        StatusPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(*LineGapPx::PANEL),
            ..default()
        },
    ));

    // One themed body-text line per vital, each with its per-line marker. The text is the
    // empty state until the update system repaints it from the selection. GTW-254 adds the
    // weapon-name line (the FIRST reader of `WeaponName`) after the existing five.
    let identity = spawn_line(&mut commands, &theme, IdentityText);
    let stance = spawn_line(&mut commands, &theme, StanceText);
    let tu = spawn_line(&mut commands, &theme, TuText);
    let hp = spawn_line(&mut commands, &theme, HpText);
    let life = spawn_line(&mut commands, &theme, LifeText);
    let weapon = spawn_line(&mut commands, &theme, WeaponNameText);

    commands
        .entity(root)
        .add_children(&[identity, stance, tu, hp, life, weapon]);
}

/// Spawns one themed body-text vitals line carrying its per-line `marker`, and
/// returns its [`Entity`].
///
/// Builds a [`Text`](bevy::prelude::Text) (starting at the [`NO_SELECTION`] empty
/// state) with the **text** sub-theme's [`TextFont`] + [`TextColor`](bevy::text::TextColor)
/// and a [`Themed(ThemeRole::Text)`](Themed) marker, so `apply_theme` re-derives the
/// body-text look on a theme change (the GTW-135 themed-text idiom; the only existing
/// body-text [`ThemeRole`] fits — no new role invented). The colors written here are
/// initial-only; `apply_theme` re-applies them every run.
fn spawn_line(commands: &mut Commands, theme: &GdtfTheme, marker: impl Bundle) -> Entity {
    commands
        .spawn((
            marker,
            Themed(ThemeRole::Text),
            Text::new(NO_SELECTION),
            TextFont {
                font: theme.text.font.clone(),
                font_size: *theme.text.font_size_pt,
                ..default()
            },
            UiTextColor(*theme.text.text_color),
        ))
        .id()
}

/// Despawns the status panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`StatusPanelRoot`] entity (and so its line children) so
/// the panel is gone the moment the battle leaves `BattleRunning` — battle-scoped
/// lifecycle. The battlescape neighborhood cleans up explicitly on `OnExit` rather
/// than via `DespawnOnExit` markers, so this mirrors the sibling action-bar.
/// Param-only (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<StatusPanelRoot>>`.
pub(in crate::scenes::running::game::battlescape) fn despawn_status_panel(
    mut commands: Commands,
    panels: Query<Entity, With<StatusPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}

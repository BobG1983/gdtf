//! Spawns + despawns the battle action-bar (GTW-228 / GTW-48 S9 / 222c).
//!
//! [`spawn_action_bar`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] node tree on the GTW-120 UI camera: a [`spawn_panel`] box
//! anchored to the bottom of the screen holding one [`spawn_button`] per EXISTING act
//! (stance-cycle, aim-toggle, fire-mode-select, level-up, level-down), each carrying
//! its own per-act marker, plus the two DEFERRED buttons (reload, end-turn) rendered
//! as [`DisabledButton`] (their acts do not exist yet — see the marker docs).
//!
//! [`despawn_action_bar`] runs `OnExit(BattleScapeState::BattleRunning)` and
//! recursively despawns the whole bar by its [`ActionBarRoot`] marker, so the bar is
//! battle-scoped: present only during the live tactical layer, gone the moment the
//! battle leaves `BattleRunning`. The battlescape neighborhood uses explicit
//! `OnExit` cleanup (not `DespawnOnExit` state-scoping), so this mirrors that.
//!
//! The bar renders on the UI camera for free: a `bevy_ui` [`Node`]/[`Button`] tree
//! with NO [`RenderLayers`](bevy::camera::visibility::RenderLayers) is routed by
//! `bevy_ui` to the highest-order camera = the GTW-120 UI camera
//! (`bevy-traps.md` #6) — NO `Pickable`/`IsDefaultUiCamera`/picking plugin needed.

use bevy::{prelude::*, ui::Val};
use gdtf_ui::{ButtonLabel, DisabledButton, spawn_button, spawn_panel, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::action_bar::components::{
    ActionBarRoot, AimToggleButton, EndTurnButton, FireModeSelectButton, FleeButton,
    LevelDownButton, LevelUpButton, ReloadButton, StanceCycleButton,
};

/// Horizontal gap between the action-bar's buttons, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it is
/// layout spacing, not a theme color/size, so it is set on the bar [`Node`] directly
/// (the theme owns palette + font, not inter-child layout — the menu `ColumnGapPx`
/// precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct BarGapPx(f32);

impl BarGapPx {
    /// The action-bar's inter-button gap: 8 px (the menu column's separation order).
    const BAR: Self = Self(8.0);
}

/// Builds the themed action-bar on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is
/// absent (`bevy-traps.md` #1) — in the running app the theme is present by the time
/// a battle starts (it loads in `Load`). With the theme present it:
///
/// 1. Spawns the bar root via [`spawn_panel`] (a `Themed(Panel)` box), tagged
///    [`ActionBarRoot`], laid out as a horizontal flex row anchored to the bottom-centre
///    of the screen with an inter-button gap.
/// 2. Spawns one [`spawn_button`] per EXISTING act — stance-cycle, aim-toggle,
///    fire-mode-select, level-up, level-down — each carrying its per-act marker so the
///    action systems' per-marker queries stay disjoint (the GTW-122 precedent).
/// 3. Spawns the two DEFERRED buttons (reload, end-turn) as [`DisabledButton`] (their
///    sim acts do not exist yet — they emit no intent; see the marker docs).
/// 4. Spawns the ENABLED [`FleeButton`] (GTW-240) — an app/lifecycle button (NO
///    `DisabledButton`) whose press ends the persisting battle via the dedicated
///    `flee_button_pressed` handler, not the sim intent seam (flee is not a sim act).
/// 5. Parents every button under the bar root.
///
/// The buttons render on the GTW-120 UI camera (a `bevy_ui` tree, no `RenderLayers`),
/// and are `Themed(Button)` for free (`spawn_button` attaches the marker). Param-only
/// (`bevy-traps.md` #7): [`Commands`] for the spawns + the theme read.
pub(in crate::scenes::running::game::battlescape) fn spawn_action_bar(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed bar. The running app
        // always has it by the time a battle starts (the menu-spawn precedent).
        return;
    };

    // ROOT: the bottom-anchored bar box. `spawn_panel` paints the panel look; we add a
    // horizontal row layout anchored to the bottom-centre via absolute positioning, plus
    // the inter-button gap. The layout fields survive `apply_theme` (it overrides only the
    // theme-owned border / radius / padding for the Panel role — the menu panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        ActionBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Percent(0.0),
            right: Val::Percent(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(*BarGapPx::BAR),
            ..default()
        },
    ));

    // One button per EXISTING act, each with its per-act marker. Captions are short act
    // names; the live theme styles them.
    let stance = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Stance"),
        StanceCycleButton,
    );
    let aim = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Aim"),
        AimToggleButton,
    );
    let fire_mode = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Fire Mode"),
        FireModeSelectButton,
    );
    let level_up = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Level +"),
        LevelUpButton,
    );
    let level_down = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Level -"),
        LevelDownButton,
    );

    // The two DEFERRED buttons — DisabledButton so they are painted inert and skipped by
    // the action layer (the `Without<DisabledButton>` filter). They emit no intent.
    let reload = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Reload"),
        (ReloadButton, DisabledButton),
    );
    let end_turn = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("End Turn"),
        (EndTurnButton, DisabledButton),
    );

    // The flee button — an ENABLED app/lifecycle button (NO DisabledButton, unlike the two
    // deferred buttons above). Its press ends the persisting battle via the dedicated
    // `flee_button_pressed` handler (GTW-240), NOT the sim intent seam (flee is not a sim act).
    let flee = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Flee battle"),
        FleeButton,
    );

    // Parent every button under the bar root, left-to-right.
    commands.entity(root).add_children(&[
        stance, aim, fire_mode, level_up, level_down, reload, end_turn, flee,
    ]);
}

/// Despawns the action-bar on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`ActionBarRoot`] entity (and so its button children) so the
/// bar is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle. The
/// battlescape neighborhood cleans up explicitly on `OnExit` rather than via
/// `DespawnOnExit` markers, so this mirrors that. Param-only (`bevy-traps.md` #7):
/// [`Commands`] + a `Query<Entity, With<ActionBarRoot>>`.
pub(in crate::scenes::running::game::battlescape) fn despawn_action_bar(
    mut commands: Commands,
    bars: Query<Entity, With<ActionBarRoot>>,
) {
    for bar in &bars {
        commands.entity(bar).despawn();
    }
}

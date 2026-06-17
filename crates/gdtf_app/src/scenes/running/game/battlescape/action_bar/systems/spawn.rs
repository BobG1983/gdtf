//! Spawns + despawns the battle action-bar (GTW-228 / GTW-48 S9 / 222c).
//!
//! [`spawn_action_bar`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] node tree on the GTW-120 UI camera: a [`spawn_panel`] box
//! anchored to the bottom of the screen holding the controls — a vertical Stance
//! 3-toggle sub-panel (Stand / Kneel / Prone, GTW-267), the aim-toggle button, a vertical
//! Mode toggle sub-panel (its per-mode toggles built on selection, GTW-265), and the
//! level-up / level-down buttons — each carrying its own marker, plus the two DEFERRED
//! buttons (reload, end-turn) rendered as [`DisabledButton`] (their acts do not exist yet
//! — see the marker docs).
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

use crate::scenes::running::game::battlescape::action_bar::{
    components::{
        ActionBarRoot, AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ReloadButton, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    },
    systems::mode_panel::spawn_mode_panel,
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

/// Vertical gap between the Stance sub-panel's toggle buttons, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): the
/// vertical stance column's inter-toggle spacing, distinct from the bar's horizontal
/// [`BarGapPx`] (the `mode_panel` `ModeGapPx` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct StanceGapPx(f32);

impl StanceGapPx {
    /// The Stance sub-panel's inter-toggle gap: 4 px (a tight stacked toggle column).
    const PANEL: Self = Self(4.0);
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
/// 2. Spawns the vertical Stance 3-toggle sub-panel (Stand / Kneel / Prone, GTW-267), the
///    aim-toggle button, the (initially-empty) Mode toggle sub-panel (GTW-265), and the
///    level-up / level-down buttons — each carrying its own marker so the action systems'
///    per-marker queries stay disjoint (the GTW-122 precedent).
/// 3. Spawns the two DEFERRED buttons (reload, end-turn) as [`DisabledButton`] (their
///    sim acts do not exist yet — they emit no intent; see the marker docs).
/// 4. Spawns the ENABLED [`FleeButton`] (GTW-240) — an app/lifecycle button (NO
///    `DisabledButton`) whose press ends the persisting battle via the dedicated
///    `flee_button_pressed` handler, not the sim intent seam (flee is not a sim act).
/// 5. Parents every control under the bar root.
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
            // GTW-272: FIT CONTENTS vertically and grow UPWARD from the bottom margin.
            // `height: Auto` sizes the bar to its tallest child (the 3-button Stance /
            // Mode columns) rather than a fixed/collapsed height, and `align_items:
            // FlexEnd` anchors every child's BOTTOM edge to the bar's bottom (= the
            // screen's bottom via `bottom: 0`), so a tall column grows upward and stays
            // fully on-screen — the old `Center` centred a tall column on the bar's
            // mid-line, pushing its bottom off the bottom edge (the Stance-clip bug). This
            // is fixed in the bar layout, NOT gdtf_ui's generic `box_node` height.
            height: Val::Auto,
            align_items: AlignItems::FlexEnd,
            column_gap: Val::Px(*BarGapPx::BAR),
            ..default()
        },
    ));
    // GTW-271 — the GTW-262 `Interaction::default()` on the bar ROOT is REMOVED: it made
    // `bevy_ui`'s `ui_focus_system` mark the whole bar area `Hovered` (the "green bar"
    // hover-paint), and the camera pan no longer needs it to be suppressed over the bar —
    // the presenter's edge-pan now gates on the cursor being INSIDE the world-map VIEWPORT
    // rect (the bar sits in the margin, outside that rect), so it never pans beneath the bar.

    // STANCE sub-panel (GTW-267): a vertical `spawn_panel` column holding the three
    // mutually-exclusive stance toggles in a sensible height order (Stand, Kneel, Prone).
    // `sync_stance_buttons_active` marks the current one ActiveButton; each press pushes a
    // DIRECT `ActIntent::SetStance` for its posture.
    let stance_panel = spawn_panel(&mut commands, &theme);
    commands.entity(stance_panel).insert((
        StancePanelRoot,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(*StanceGapPx::PANEL),
            // GTW-272: FIT CONTENTS vertically — the column sizes to its three stacked
            // toggles (no fixed/min height), so the full Stance column is on-screen.
            height: Val::Auto,
            ..default()
        },
    ));
    let stand = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Stand"),
        StanceStandingButton,
    );
    let kneel = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Kneel"),
        StanceKneelingButton,
    );
    let prone = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Prone"),
        StanceProneButton,
    );
    commands
        .entity(stance_panel)
        .add_children(&[stand, kneel, prone]);

    let aim = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Aim"),
        AimToggleButton,
    );
    // MODE sub-panel (GTW-265): a vertical `spawn_panel` column whose per-mode toggle
    // children are (re)built to the SELECTED weapon's offered modes by
    // `rebuild_mode_buttons`. Empty at spawn (no selection yet); filled on the first
    // selection change. This REPLACED the GTW-254 popup picker (no modal, no scrim).
    let mode_panel = spawn_mode_panel(&mut commands, &theme);
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

    // Parent every control under the bar root, left-to-right. The Stance + Mode sub-panels
    // are nested column children of the horizontal bar row. Mode precedes Stance (GTW-272):
    // the Mode sub-panel renders to the LEFT of the Stance sub-panel.
    commands.entity(root).add_children(&[
        mode_panel,
        stance_panel,
        aim,
        level_up,
        level_down,
        reload,
        end_turn,
        flee,
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

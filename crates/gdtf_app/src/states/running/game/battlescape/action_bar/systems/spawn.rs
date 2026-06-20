//! Spawns + despawns the battle action-bar (GTW-228 / GTW-48 S9 / 222c).
//!
//! [`spawn_action_bar`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] node tree on the GTW-120 UI camera: a [`spawn_panel`] box
//! anchored to the TOP-CENTRE of the screen (GTW-298 — the contextual-control strip floats over
//! the map at the top, per the mockup) holding the controls — a vertical Stance
//! 3-toggle sub-panel (Stand / Kneel / Prone, GTW-267), the aim-toggle button, a vertical
//! Mode toggle sub-panel (its per-mode toggles built on selection, GTW-265), the
//! level-up / level-down buttons, and the end-turn button — each carrying its own marker.
//! GTW-309 made the end-turn button LIVE: it is now an ENABLED button that pushes
//! `ActIntent::EndTurn` on a press (no longer a disabled placeholder).
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
use gdtf_ui::{ButtonLabel, spawn_button, spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::action_bar::components::{
    ActionBarRoot, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
};

/// Horizontal gap between the action-bar's buttons, as a fraction of the viewport WIDTH
/// ([`Val::Vw`](bevy::ui::Val::Vw)).
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it is
/// layout spacing, not a theme color/size, so it is set on the bar [`Node`] directly
/// (the theme owns palette + font, not inter-child layout — the menu `ColumnGapPx`
/// precedent). A horizontal gap, so the unit is `Vw` (`ui-responsive-not-px`).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct BarGapVw(f32);

impl BarGapVw {
    /// The action-bar's inter-button gap: 0.625 vw (8 px at the 1280-wide reference window).
    const BAR: Self = Self(0.625);
}

/// Builds the themed action-bar on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is
/// absent (`bevy-traps.md` #1) — in the running app the theme is present by the time
/// a battle starts (it loads in `Load`). With the theme present it:
///
/// 1. Spawns the bar ROOT as a TRANSPARENT full-window-width centering WRAPPER (no `Themed` /
///    border / background), tagged [`ActionBarRoot`], anchored to the TOP of the screen and
///    spanning the full window width with `justify_content: Center` — so its single child (the
///    compact panel) is centred at the top-middle of the window (D-C, screenshot review
///    2026-06-18: the strip shrink-wraps + sits top-centre rather than spanning the full width).
/// 2. Spawns the visible compact PANEL via [`spawn_panel`] (a `Themed(Panel)` box) as a
///    `width: Auto` / `height: Auto` horizontal flex row that FITS its four buttons, with an
///    inter-button gap — the bordered strip the player sees.
/// 3. Spawns the level-up / level-down buttons — each carrying its own marker so the action
///    systems' per-marker queries stay disjoint (the GTW-122 precedent).
/// 4. Spawns the LIVE end-turn button (GTW-309) — an ENABLED button (NO `DisabledButton`)
///    whose press pushes `ActIntent::EndTurn` on the shared 222a seam; see the marker docs.
/// 5. Spawns the ENABLED [`FleeButton`] (GTW-240; labelled `"Flee"`, D-D) — an app/lifecycle
///    button (NO `DisabledButton`) whose press ends the persisting battle via the dedicated
///    `flee_button_pressed` handler, not the sim intent seam (flee is not a sim act).
/// 6. Parents the four controls under the compact panel, then the panel under the wrapper root.
///
/// GTW-298 RELOCATED the Stance 3-toggle sub-panel, the Aim toggle, and the Firemode 3-toggle
/// sub-panel OUT of this bar and INTO the weapon cluster (the weapon-panel module spawns them
/// into its Stance / Aim / Firemode panels via the shared `spawn_stance_panel` /
/// `spawn_mode_panel` constructors + the `AimToggleButton` marker). The press → intent router
/// (`action_bar_button_intents`) and the active-mark syncs are UNCHANGED — they query the
/// stance / aim markers parent-agnostically, so the relocated controls keep working. This bar
/// now hosts only the level + end-turn + flee controls.
///
/// The buttons render on the GTW-120 UI camera (a `bevy_ui` tree, no `RenderLayers`),
/// and are `Themed(Button)` for free (`spawn_button` attaches the marker). Param-only
/// (`bevy-traps.md` #7): [`Commands`] for the spawns + the theme read.
pub(in crate::states::running::game::battlescape) fn spawn_action_bar(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed bar. The running app
        // always has it by the time a battle starts (the menu-spawn precedent).
        return;
    };

    // ROOT: a TRANSPARENT full-window-width centering WRAPPER anchored to the TOP of the screen
    // (D-C, 2026-06-18 screenshot review). It is NOT itself the visible compact panel — it is an
    // invisible absolute row spanning the full window width whose ONLY job is to horizontally
    // CENTRE its single child (the fit-content button panel) via `justify_content: Center`. This
    // is how a fit-content node is centred under absolute positioning in `bevy_ui` (there is no
    // CSS `translateX(-50%)`): the panel sizes to its 4 buttons, and the full-width parent centres
    // it at the top-middle of the window. It carries NO `Themed` / border / background, so only
    // the inner panel reads as a bordered box. It carries the [`ActionBarRoot`] marker, so the
    // `OnExit` recursive despawn tears down the wrapper + the panel + the buttons by this one
    // marker. It is an absolute overlay (like the corner status / hover panels), so it does NOT
    // inset the world map — `set_world_viewport` measures only the bottom bar.
    let root = commands
        .spawn((
            ActionBarRoot,
            Node {
                position_type: PositionType::Absolute,
                // Anchor the centering row to the TOP, spanning the full window WIDTH so its
                // `justify_content: Center` can centre the compact panel at the top-middle
                // (GTW-298 kept the top-of-screen float; this fix shrink-wraps + centres it).
                top: Val::ZERO,
                left: Val::Vw(0.0),
                width: Val::Vw(100.0),
                // FIT CONTENTS vertically (the wrapper is as tall as the panel) and anchor the
                // panel to the top edge.
                height: Val::Auto,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                ..default()
            },
        ))
        .id();
    // GTW-271 — the GTW-262 `Interaction::default()` on the bar ROOT is REMOVED: it made
    // `bevy_ui`'s `ui_focus_system` mark the whole bar area `Hovered` (the "green bar"
    // hover-paint), and the camera pan no longer needs it to be suppressed over the bar —
    // the presenter's edge-pan now gates on the cursor being INSIDE the world-map VIEWPORT
    // rect (the bar sits in the margin, outside that rect), so it never pans beneath the bar.

    // PANEL: the visible compact bordered button row (D-C). `spawn_panel` paints the panel look;
    // we overwrite its [`Node`] with a `width: Auto` / `height: Auto` row that FITS its 4 buttons
    // (not the old full-window stretch), plus the inter-button gap. As the single child of the
    // full-width wrapper above, it is centred at the top-middle. The layout fields survive
    // `apply_theme` (it overrides only the theme-owned border / radius / padding for the Panel
    // role — the menu panel precedent).
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert(Node {
        // FIT CONTENTS on BOTH axes — the panel is exactly as wide + tall as its 4 buttons, so it
        // reads as a compact framed strip rather than a full-width bar (D-C: width Auto, not 100%).
        width: Val::Auto,
        height: Val::Auto,
        column_gap: Val::Vw(*BarGapVw::BAR),
        ..default()
    });

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

    // GTW-309: the end-turn button is now LIVE (the turn-cycle engine landed). It is an
    // ENABLED action-bar button (NO `DisabledButton`, so the `Without<DisabledButton>`
    // action filter now INCLUDES it) whose press pushes `ActIntent::EndTurn` onto the shared
    // 222a seam — the same fieldless GLOBAL turn signal the keyboard surface pushes.
    let end_turn = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("End Turn"),
        EndTurnButton,
    );

    // The flee button — an ENABLED app/lifecycle button (NO DisabledButton, unlike the
    // deferred end-turn button above). Its press ends the persisting battle via the dedicated
    // `flee_button_pressed` handler (GTW-240), NOT the sim intent seam (flee is not a sim act).
    let flee = spawn_button(&mut commands, &theme, ButtonLabel::new("Flee"), FleeButton);

    // Parent the four controls under the compact PANEL (left-to-right), then parent the panel
    // under the full-width centering WRAPPER (root) so it is centred at the top-middle (D-C).
    commands
        .entity(panel)
        .add_children(&[level_up, level_down, end_turn, flee]);
    commands.entity(root).add_children(&[panel]);
}

/// Despawns the action-bar on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`ActionBarRoot`] entity (and so its button children) so the
/// bar is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle. The
/// battlescape neighborhood cleans up explicitly on `OnExit` rather than via
/// `DespawnOnExit` markers, so this mirrors that. Param-only (`bevy-traps.md` #7):
/// [`Commands`] + a `Query<Entity, With<ActionBarRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_action_bar(
    mut commands: Commands,
    bars: Query<Entity, With<ActionBarRoot>>,
) {
    for bar in &bars {
        commands.entity(bar).despawn();
    }
}

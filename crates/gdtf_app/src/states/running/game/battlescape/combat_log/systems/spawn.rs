//! Spawns + despawns the battlescape combat-text LOG container (GTW-328, slice 3,
//! bottom-left, ABOVE the weapon panel).
//!
//! [`spawn_combat_log`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds the empty
//! [`CombatLogRoot`] container — a themed [`gdtf_ui`](gdtf_ui) panel on the GTW-120 UI camera,
//! anchored BOTTOM-LEFT and sitting just ABOVE the bottom bar (so it never overlaps the
//! weapon panel inside the bar), laid out as a RESPONSIVE `flex column` (window-relative `Vw`
//! width + `Vh` bottom anchor; the height is intrinsic to the line count). It carries
//! [`GlobalZIndex(COMBAT_LOG_Z)`](bevy::ui::GlobalZIndex) — ABOVE the opaque bottom bar's
//! `GlobalZIndex(10)` so the log is not occluded (`bevy-traps.md` #8); the lines themselves are
//! appended by [`update_combat_log`](super::update::update_combat_log) on each combat event.
//!
//! The lines append NEWEST AT THE BOTTOM: a fresh event is `add_children`'d (appended) under
//! the column root, so the most recent line sits lowest and older lines drift up — the natural
//! reading order for a bottom-anchored log (the eye rests at the bottom edge, nearest the
//! action). The overflow trim despawns the OLDEST (the first child) when the count exceeds the
//! tuned max.
//!
//! [`despawn_combat_log`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the log by its [`CombatLogRoot`] marker — battle-scoped, mirroring the sibling
//! status panel / weapon panel lifecycle.

use bevy::{
    color::Alpha,
    prelude::*,
    ui::{BackgroundColor, FlexDirection, GlobalZIndex, Node, Overflow, PositionType, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::BOTTOM_BAR_H_VH,
    combat_log::{
        components::{CombatLogRoot, PanelHeightAnim},
        tuning::CombatLogTuning,
    },
};

/// The combat log's stacking order ([`GlobalZIndex`] — the higher, the nearer the viewer).
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`] (the framework carve-out,
/// not a domain value). Set to `11` — strictly ABOVE the opaque bottom bar's `GlobalZIndex(10)`
/// so the log (which sits just above the bar's top edge, bottom-left) is NEVER painted over by
/// the bar (`bevy-traps.md` #8: an opaque higher-z sibling occludes a no-z panel). It matches
/// the on-bar weapon cluster's z (`11`) — the two never overlap (the log is above the bar, the
/// weapon panel inside it), so a shared z is deterministic and unambiguous.
const COMBAT_LOG_Z: i32 = 11;

/// The combat log panel's TRANSLUCENT background alpha so the dotted map reads through behind
/// the recent-event lines (a light HUD overlay, NOT the opaque bottom bar).
///
/// A framework-plumbing `const` fed to the panel fill's alpha (the carve-out). Kept light so
/// the log is a glanceable overlay that does not blot out the map below it.
const COMBAT_LOG_BG_ALPHA: f32 = 0.45;

/// Builds the empty combat-log container on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle starts; the
/// weapon-panel / status-panel precedent). The [`CombatLogTuning`] is read as
/// `Option<Res<CombatLogTuning>>` and DEFAULTED if absent — the only tuning the spawn needs is
/// the panel width, and a missing-RON / not-yet-resolved tuning should still produce a log at the
/// sane default width (the `OnEnter` spawn must not depend on the async RON resolve having
/// completed first). With the theme present it spawns a [`spawn_panel`](gdtf_ui::spawn_panel)
/// themed box (bordered, panel fill / radius re-painted by `apply_theme`) and overwrites its
/// [`Node`] with the log layout: a [`PositionType::Absolute`] `flex column` anchored bottom-left
/// just ABOVE the bottom bar ([`bottom: Val::Vh(BOTTOM_BAR_H_VH)`](bevy::ui::Val)), at the tuned
/// [`panel_width_vw`](super::super::tuning::CombatLogTuning::panel_width_vw) window-relative
/// width (responsive — `Vw`/`Vh` only, `ui-responsive-not-px`). The fill alpha is forced to
/// [`COMBAT_LOG_BG_ALPHA`] so the map reads through (a light overlay). [`GlobalZIndex`]
/// [`COMBAT_LOG_Z`] keeps it above the opaque bottom bar (`bevy-traps.md` #8). The container
/// starts EMPTY — lines are appended on combat events by `update_combat_log`.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] + the theme + tuning reads.
pub(in crate::states::running::game::battlescape) fn spawn_combat_log(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    tuning: Option<Res<CombatLogTuning>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed log (the weapon-panel precedent).
        // The running app always has it by the time a battle starts.
        return;
    };
    // The tuning's RON resolve is async; default the width so the OnEnter spawn does not depend
    // on it having completed (it re-tunes live on resolve / hot-reload anyway).
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);

    let root = spawn_panel(&mut commands, &theme);
    // A LIGHT translucent fill so the dotted map reads through the log overlay (the theme's
    // panel fill is heavier); the theme still owns the border / radius (re-applied every run).
    let mut fill = *theme.panel.color;
    fill.set_alpha(COMBAT_LOG_BG_ALPHA);
    commands.entity(root).insert((
        CombatLogRoot,
        // GTW-328 slice B: the root carries an ANIMATED height (seeded 0, grows in as lines
        // appear) instead of snapping with the line count — `animate_combat_log_height` lerps it
        // toward the natural content height each frame.
        PanelHeightAnim::new(0.0),
        Node {
            position_type: PositionType::Absolute,
            // Anchored just ABOVE the bottom bar (which is BOTTOM_BAR_H_VH tall at the window
            // bottom), bottom-left — so the log floats over the map, never overlapping the
            // weapon panel inside the bar.
            bottom: Val::Vh(BOTTOM_BAR_H_VH),
            left: Val::ZERO,
            width: Val::Vw(*tuning.panel_width_vw),
            // GTW-328 slice B: an explicit ANIMATED height (logical px), lerped toward the
            // summed line height by `animate_combat_log_height` so the panel grows / shrinks
            // smoothly rather than snapping with the line count. Seeded 0 (empty log); responsive
            // width keeps the `ui-responsive-not-px` intent (height is a per-frame animated value,
            // not a fixed layout constant). `overflow: clip` so a line sliding in from just below
            // the (animating) height is masked rather than spilling past the panel edge.
            height: Val::Px(0.0),
            overflow: Overflow::clip(),
            // Newest line appended at the END (bottom) — see the module docs.
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(fill),
        // ABOVE the opaque bottom bar so the log is not occluded (`bevy-traps.md` #8).
        GlobalZIndex(COMBAT_LOG_Z),
    ));
}

/// Despawns the combat log on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`CombatLogRoot`] entity (and so all its line children) so the log
/// is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle. Param-only
/// (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<CombatLogRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_combat_log(
    mut commands: Commands,
    logs: Query<Entity, With<CombatLogRoot>>,
) {
    for log in &logs {
        commands.entity(log).despawn();
    }
}

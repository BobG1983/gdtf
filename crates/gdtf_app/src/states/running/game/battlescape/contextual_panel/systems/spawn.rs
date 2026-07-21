//! Spawns + despawns the battlescape CONTEXTUAL PANEL's root box (GTW-294; the per-act
//! buttons are spawned generically since GTW-571).
//!
//! [`spawn_contextual_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` (in the
//! spawn band's `Root` set, ordered `.after(spawn_bottom_bar)`) and builds the
//! bottom-RIGHT panel BOX only — a themed [`spawn_panel`] node ([`ContextualPanelRoot`])
//! parented as a CHILD of the [`BottomBarRoot`] container and anchored to the bar's
//! bottom-right corner with responsive units. Each registered act's generic
//! [`spawn_contextual_button`](super::spawn_contextual_button) then parents its themed
//! button under this root (the `Buttons` set), and the deterministic
//! [`order_contextual_buttons`](super::order_contextual_buttons) pass sorts the column
//! (the `Order` set). The box spawns
//! [`Visibility::Hidden`](bevy::camera::visibility::Visibility): the act-agnostic
//! `sync_panel_root_visibility` pass reveals it iff ANY act button is visible.
//!
//! ## Why a CHILD of the bottom bar (the GTW-726 detach + flicker fix)
//!
//! Before GTW-726 the panel was a BARE top-level absolute root anchored to the WINDOW's
//! bottom-right, carrying a [`GlobalZIndex`](bevy::ui::GlobalZIndex) above the bar to
//! escape the occlusion the bar's opaque fill would otherwise cause. That produced two
//! bugs: (1) DETACHED — the box floated over the map viewport in the lower-right rather
//! than reading as part of the bottom HUD (worse, `Visibility::Hidden` buttons still
//! reserve their layout row, so the eight-button column was eight rows tall regardless of
//! how few acts were offered, pushing the box up well past the bar onto the map); and
//! (2) FLICKER — because the box floated over hoverable map cells, and the Throw act's
//! offer is the cell under the cursor, the box appearing under the cursor made
//! `pick_hovered_cell`'s over-UI gate clear the hovered cell (GTW-380), which hid the box,
//! which un-cleared the cell, which showed it again — an every-frame oscillation.
//!
//! The fix (mirroring the stance panel's D4 reparent) is to make the panel a CHILD of the
//! [`BottomBarRoot`]: the bar's height insets the world viewport, so a box laid out inside
//! the bar never overlaps a hoverable map cell (killing the flicker loop), and it reads as
//! part of the bottom HUD (killing the detach). Being a child also removes the need for a
//! `GlobalZIndex` — a child renders in the bar's own stacking context, on top of the bar's
//! fill (the stance panel carries no z at all). The buttons collapse their layout via
//! [`Display::None`] when not offered (the per-act
//! [`sync_contextual_button_visibility`](super::sync_contextual_button_visibility) toggle),
//! so the column sizes to only its offered buttons and stays within the bar's height. The
//! [`CONTEXTUAL_PANEL_Z`] `GlobalZIndex` survives only as a defensive fallback for the
//! (ordering-guarded, effectively unreachable) case where the bar root is not yet present.
//!
//! [`despawn_contextual_panel`] runs `OnExit(BattleScapeState::BattleRunning)` (ordered
//! `.before(despawn_bottom_bar)`, so the root is torn down and unlinked from the bar before
//! the bar's own recursive despawn runs — no double despawn) and recursively despawns the
//! whole subtree by the [`ContextualPanelRoot`] marker (the root owns the act buttons) —
//! battle-scoped, mirroring the sibling action-bar / bottom-bar / weapon-panel lifecycle
//! (the battlescape neighborhood uses explicit `OnExit` cleanup, not `DespawnOnExit`
//! state-scoping).
//!
//! The panel renders on the GTW-120 UI camera for free: a `bevy_ui` [`Node`] /
//! [`Button`] tree with NO [`RenderLayers`](bevy::camera::visibility::RenderLayers) is
//! routed by `bevy_ui` to the highest-order camera = the UI camera (`bevy-traps.md` #6).

use bevy::{
    prelude::*,
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::{BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    contextual_panel::components::{
        CONTEXTUAL_PANEL_RIGHT_VW, CONTEXTUAL_PANEL_ROW_GAP_VH, CONTEXTUAL_PANEL_WIDTH_VW,
        CONTEXTUAL_PANEL_Z, ContextualPanelRoot,
    },
};

/// Builds the contextual panel's ROOT box on `OnEnter(BattleScapeState::BattleRunning)`
/// — the spawn band's `Root` stage; the per-act generic button spawns follow it.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle
/// starts; the action-bar / bottom-bar precedent). With the theme present it spawns the
/// panel BOX via [`spawn_panel`] (a `Themed(Panel)` box), tags it [`ContextualPanelRoot`],
/// and mutates its [`Node`] (via `entry::<Node>().and_modify`, so the `spawn_panel` box
/// border / padding survive) into the bottom-RIGHT anchor: a [`PositionType::Absolute`]
/// column inset off its parent's right + bottom edges by the responsive
/// [`CONTEXTUAL_PANEL_RIGHT_VW`] / [`BOTTOM_BAR_PAD_Y_VH`] (the same bottom inset the
/// sibling weapon + stance panels use), at the responsive width
/// [`CONTEXTUAL_PANEL_WIDTH_VW`] with `height: Auto` (fits its offered buttons — the
/// per-act toggle collapses the un-offered rows) and a [`CONTEXTUAL_PANEL_ROW_GAP_VH`]
/// inter-button gap. It spawns
/// [`Visibility::Hidden`](bevy::camera::visibility::Visibility).
///
/// GTW-726 — the root is parented as a CHILD of the [`BottomBarRoot`] container (the
/// stance-panel precedent, `weapon_panel`'s `spawn_weapon_panel`), so it is laid out
/// INSIDE the bottom panel rather than floating over the map: the bar's height insets the
/// world viewport (`set_world_viewport`), so a box within the bar never overlaps a
/// hoverable map cell. Being a child also RESOLVES the old occlusion problem WITHOUT a
/// `GlobalZIndex`: a child renders in the bar's own stacking context, on top of the bar's
/// fill (the stance panel proves it — it carries no `GlobalZIndex`). The
/// [`CONTEXTUAL_PANEL_Z`] `GlobalZIndex` survives only as a DEFENSIVE fallback for the
/// (ordering-guarded, effectively unreachable) case where the bar root is not yet present,
/// where the panel stays a top-level root and needs the z to draw over the bar — the
/// pre-GTW-726 behavior. `spawn_contextual_panel` is ordered `.after(spawn_bottom_bar)`
/// (in the plugin) so the bar root exists.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the theme read, and a read-only
/// `Query<Entity, With<BottomBarRoot>>`.
pub(in crate::states::running::game::battlescape) fn spawn_contextual_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    bottom_bar: Query<Entity, With<BottomBarRoot>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the action-bar /
        // bottom-bar precedent). The running app always has it by the time a battle starts.
        return;
    };

    // PANEL BOX: a themed panel anchored to the bottom-RIGHT of the bottom bar (the mockup's
    // contextual cluster, to the right of the Stance column). `spawn_panel` paints the panel
    // look; we MUTATE its Node via `entry::<Node>().and_modify` (rather than overwriting the
    // whole Node) so the box's `spawn_panel`-set border / padding are preserved and only the
    // positioning / layout fields are set. The layout fields survive `apply_theme` (it overrides
    // only the theme-owned border / radius / padding for the Panel role — the action-bar /
    // bottom-bar precedent). Spawned Visibility::Hidden — `sync_panel_root_visibility` reveals it
    // when any act button is visible.
    let panel = spawn_panel(&mut commands, &theme);
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.right = Val::Vw(CONTEXTUAL_PANEL_RIGHT_VW);
            // Anchor a bottom-padding's worth up from the bar's bottom edge — the SAME
            // `bottom` the sibling weapon / stance panels use, so all three bottom-panel
            // clusters share a baseline inside the bar.
            node.bottom = Val::Vh(BOTTOM_BAR_PAD_Y_VH);
            node.width = Val::Vw(CONTEXTUAL_PANEL_WIDTH_VW);
            // FIT the stacked OFFERED buttons vertically — the panel is exactly as tall as its
            // currently-shown buttons + the inter-button gaps (the un-offered buttons collapse
            // via `Display::None`), so the column stays within the bar's height.
            node.height = Val::Auto;
            // Stack the act buttons in a column with a responsive inter-button gap.
            node.flex_direction = FlexDirection::Column;
            node.row_gap = Val::Vh(CONTEXTUAL_PANEL_ROW_GAP_VH);
        });
    commands.entity(panel).insert((
        ContextualPanelRoot,
        // Hidden by default — the root visibility pass reveals the panel only when at
        // least one act is offered (`ui-mutate-not-respawn`: a Visibility toggle in place).
        Visibility::Hidden,
    ));

    // GTW-726: parent the panel INSIDE the bottom bar (the stance-panel precedent) so it is
    // laid out within the bottom panel, not floating over the map — and so a child of the bar
    // renders above the bar fill with NO `GlobalZIndex`. If the bar root is somehow absent
    // (defensive — `spawn_contextual_panel` is ordered `.after(spawn_bottom_bar)`, so it should
    // not be), fall back to the pre-GTW-726 top-level root carrying the `GlobalZIndex` that keeps
    // it above the opaque bar.
    match bottom_bar.iter().next() {
        Some(bar) => {
            commands.entity(bar).add_children(&[panel]);
        }
        None => {
            commands
                .entity(panel)
                .insert(GlobalZIndex(CONTEXTUAL_PANEL_Z));
        }
    }
}

/// Despawns the contextual panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`ContextualPanelRoot`] entity (and so its act-button
/// children) so the whole subtree is gone the moment the battle leaves `BattleRunning`
/// — battle-scoped lifecycle. The battlescape neighborhood cleans up explicitly on
/// `OnExit` rather than via `DespawnOnExit` markers, so this mirrors that. Param-only
/// (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<ContextualPanelRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_contextual_panel(
    mut commands: Commands,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

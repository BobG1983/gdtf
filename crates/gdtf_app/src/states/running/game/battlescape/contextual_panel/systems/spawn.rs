//! Spawns + despawns the battlescape CONTEXTUAL PANEL's root box (GTW-294; the per-act
//! buttons are spawned generically since GTW-571).
//!
//! [`spawn_contextual_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` (in the
//! spawn band's `Root` set) and builds the bottom-RIGHT panel BOX only — a BARE themed
//! [`spawn_panel`] node ([`ContextualPanelRoot`]) anchored to the window's bottom-right
//! corner with responsive units. Each registered act's generic
//! [`spawn_contextual_button`](super::spawn_contextual_button) then parents its themed
//! button under this root (the `Buttons` set), and the deterministic
//! [`order_contextual_buttons`](super::order_contextual_buttons) pass sorts the column
//! (the `Order` set). The box spawns
//! [`Visibility::Hidden`](bevy::camera::visibility::Visibility): the act-agnostic
//! `sync_panel_root_visibility` pass reveals it iff ANY act button is visible.
//!
//! ## Why a `GlobalZIndex` above the bottom bar (the GTW-294 occlusion fix)
//!
//! The contextual panel is anchored bottom-RIGHT, which lands INSIDE the bottom bar's
//! full-width opaque footprint. The bottom bar is a `GlobalZIndex(10)` opaque panel;
//! with NO `GlobalZIndex` (default `0`) this panel drew BEHIND it and rendered zero
//! pixels — the GTW-294 occlusion bug, proven by a runtime probe (the panel resolved
//! the SAME UI camera as the bar, was parented, on-screen, sized, and `Visible`, yet
//! nothing rendered because the opaque bar painted over it). The fix mirrors the
//! bottom-bar / weapon-panel precedent: a BARE absolute root carrying
//! [`GlobalZIndex`](bevy::ui::GlobalZIndex)`(`[`CONTEXTUAL_PANEL_Z`]`)` (`20`), strictly
//! above the bar (`10`) and the on-bar weapon + stance cluster (`11`), so the whole
//! panel subtree stacks on top of the bar. A bare visible root resolves the UI camera
//! fine (the bottom bar proves it), so NO wrapper is needed.
//!
//! [`despawn_contextual_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and
//! recursively despawns the whole subtree by the [`ContextualPanelRoot`] marker (the
//! root owns the act buttons) — battle-scoped, mirroring the sibling action-bar /
//! bottom-bar / weapon-panel lifecycle (the battlescape neighborhood uses explicit
//! `OnExit` cleanup, not `DespawnOnExit` state-scoping).
//!
//! The panel renders on the GTW-120 UI camera for free: a `bevy_ui` [`Node`] /
//! [`Button`] tree with NO [`RenderLayers`](bevy::camera::visibility::RenderLayers) is
//! routed by `bevy_ui` to the highest-order camera = the UI camera (`bevy-traps.md` #6).

use bevy::{
    prelude::*,
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::contextual_panel::components::{
    CONTEXTUAL_PANEL_BOTTOM_VH, CONTEXTUAL_PANEL_RIGHT_VW, CONTEXTUAL_PANEL_ROW_GAP_VH,
    CONTEXTUAL_PANEL_WIDTH_VW, CONTEXTUAL_PANEL_Z, ContextualPanelRoot,
};

/// Builds the contextual panel's ROOT box on `OnEnter(BattleScapeState::BattleRunning)`
/// — the spawn band's `Root` stage; the per-act generic button spawns follow it.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle
/// starts; the action-bar / bottom-bar precedent). With the theme present it spawns the
/// panel BOX via [`spawn_panel`] (a `Themed(Panel)` box) as the BARE root, tags it
/// [`ContextualPanelRoot`], and mutates its [`Node`] (via `entry::<Node>().and_modify`,
/// so the `spawn_panel` box border / padding survive) into the bottom-RIGHT anchor: a
/// [`PositionType::Absolute`] column inset off the WINDOW's right + bottom edges by the
/// responsive [`CONTEXTUAL_PANEL_RIGHT_VW`] / [`CONTEXTUAL_PANEL_BOTTOM_VH`], at the
/// responsive width [`CONTEXTUAL_PANEL_WIDTH_VW`] with `height: Auto` (fits its
/// buttons) and a [`CONTEXTUAL_PANEL_ROW_GAP_VH`] inter-button gap. It carries
/// [`GlobalZIndex`](bevy::ui::GlobalZIndex)`(`[`CONTEXTUAL_PANEL_Z`]`)` (the GTW-294
/// occlusion fix — see the module doc) and spawns
/// [`Visibility::Hidden`](bevy::camera::visibility::Visibility).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn + the theme read.
pub(in crate::states::running::game::battlescape) fn spawn_contextual_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the action-bar /
        // bottom-bar precedent). The running app always has it by the time a battle starts.
        return;
    };

    // PANEL BOX: a themed panel anchored to the window's BOTTOM-RIGHT corner (the mockup's
    // contextual cluster, to the right of the Stance column) — the BARE root (the bottom-bar
    // precedent: a bare absolute root resolves the UI camera fine, no wrapper needed).
    // `spawn_panel` paints the panel look; we MUTATE its Node via `entry::<Node>().and_modify`
    // (rather than overwriting the whole Node) so the box's `spawn_panel`-set border / padding are
    // preserved and only the positioning / layout fields are set. The layout fields survive
    // `apply_theme` (it overrides only the theme-owned border / radius / padding for the Panel
    // role — the action-bar / bottom-bar precedent). Spawned Visibility::Hidden —
    // `sync_panel_root_visibility` reveals it when any act button is visible.
    let panel = spawn_panel(&mut commands, &theme);
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.right = Val::Vw(CONTEXTUAL_PANEL_RIGHT_VW);
            node.bottom = Val::Vh(CONTEXTUAL_PANEL_BOTTOM_VH);
            node.width = Val::Vw(CONTEXTUAL_PANEL_WIDTH_VW);
            // FIT the stacked buttons vertically — the panel is exactly as tall as its
            // buttons + the inter-button gaps.
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
        // Stack the whole panel subtree ABOVE the opaque bottom bar (GlobalZIndex 10) it overlaps,
        // and above the on-bar weapon + stance cluster (GlobalZIndex 11). Without this the panel
        // resolved the right camera + position + size yet rendered nothing — the bar painted over
        // it (the GTW-294 occlusion bug). GlobalZIndex propagates to the button children.
        GlobalZIndex(CONTEXTUAL_PANEL_Z),
    ));
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

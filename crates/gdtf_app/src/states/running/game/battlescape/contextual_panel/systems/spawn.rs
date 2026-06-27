//! Spawns + despawns the battlescape CONTEXTUAL PANEL (GTW-294).
//!
//! [`spawn_contextual_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds the
//! bottom-RIGHT contextual cluster (per the `docs/ui_mockups/battlescape_mockup.png`
//! bottom-right corner — "Contextual Buttons go Here", to the right of the Stance column). It
//! spawns a BARE themed [`spawn_panel`] box ([`ContextualPanelRoot`]) anchored to the window's
//! bottom-right corner with responsive units, holding three themed [`spawn_button`] buttons
//! stacked in a column — **Execute**, **Stabilize**, **Open Door** — each carrying its marker. The
//! panel box AND every button spawn [`Visibility::Hidden`](bevy::render::view::Visibility): this
//! system only builds the tree. The live `detect_contextual_targets` system (registered by the
//! plugin) fills [`ContextualTargets`] each update and toggles the box + each button's
//! `Visibility` IN PLACE when a valid downed neighbour is in reach, and `contextual_button_intents`
//! routes a press to the shared act-intent seam. Execute + Stabilize are live; Open Door stays
//! hidden (no sim verb yet — see below).
//!
//! ## Why a `GlobalZIndex` above the bottom bar (the GTW-294 occlusion fix)
//!
//! The contextual panel is anchored bottom-RIGHT, which lands INSIDE the bottom bar's full-width
//! opaque footprint. The bottom bar is a `GlobalZIndex(10)` opaque panel; with NO `GlobalZIndex`
//! (default `0`) this panel drew BEHIND it and rendered zero pixels — the GTW-294 occlusion bug,
//! proven by a runtime probe (the panel resolved the SAME UI camera as the bar, was parented,
//! on-screen, sized, and `Visible`, yet nothing rendered because the opaque bar painted over it).
//! The fix mirrors the bottom-bar / weapon-panel precedent: a BARE absolute root carrying a
//! [`GlobalZIndex`](bevy::ui::GlobalZIndex) — here
//! [`CONTEXTUAL_PANEL_Z`] (`20`), strictly above the bar (`10`) and the on-bar weapon + stance
//! cluster (`11`) — so the whole panel subtree (root box + buttons) stacks on top of the bar. A
//! bare visible root resolves the UI camera fine (the bottom bar proves it), so NO wrapper is
//! needed.
//!
//! [`despawn_contextual_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole subtree by the [`ContextualPanelRoot`] marker (the root owns the three
//! buttons) — battle-scoped, mirroring the sibling action-bar / bottom-bar / weapon-panel
//! lifecycle (the battlescape neighborhood uses explicit `OnExit` cleanup, not `DespawnOnExit`
//! state-scoping).
//!
//! The panel renders on the GTW-120 UI camera for free: a `bevy_ui` [`Node`] / [`Button`] tree
//! with NO [`RenderLayers`](bevy::camera::visibility::RenderLayers) is routed by `bevy_ui` to
//! the highest-order camera = the UI camera (`bevy-traps.md` #6).

use bevy::{
    prelude::*,
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{ButtonLabel, spawn_button, spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::contextual_panel::components::{
    CONTEXTUAL_PANEL_BOTTOM_VH, CONTEXTUAL_PANEL_RIGHT_VW, CONTEXTUAL_PANEL_ROW_GAP_VH,
    CONTEXTUAL_PANEL_WIDTH_VW, CONTEXTUAL_PANEL_Z, ContextualPanelRoot, ExecuteButton,
    OpenDoorButton, StabilizeButton,
};

/// Builds the contextual-panel tree on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle starts;
/// the action-bar / bottom-bar precedent). With the theme present it:
///
/// 1. Spawns the panel BOX via [`spawn_panel`] (a `Themed(Panel)` box) as the BARE root, tags it
///    [`ContextualPanelRoot`], and mutates its [`Node`] (via `entry::<Node>().and_modify`, so the
///    `spawn_panel` box border / padding survive) into the bottom-RIGHT anchor: a
///    [`PositionType::Absolute`] column inset off the WINDOW's right + bottom edges by the
///    responsive [`CONTEXTUAL_PANEL_RIGHT_VW`] / [`CONTEXTUAL_PANEL_BOTTOM_VH`], at the responsive
///    width [`CONTEXTUAL_PANEL_WIDTH_VW`] with `height: Auto` (fits its buttons) and a
///    [`CONTEXTUAL_PANEL_ROW_GAP_VH`] inter-button gap. It carries
///    [`GlobalZIndex`](bevy::ui::GlobalZIndex)`(`[`CONTEXTUAL_PANEL_Z`]`)` so the whole panel
///    subtree stacks ABOVE the opaque bottom bar it overlaps (the GTW-294 occlusion fix — see the
///    module doc). It is spawned [`Visibility::Hidden`](bevy::render::view::Visibility) — the live
///    `detect_contextual_targets` system reveals it when a downed neighbour is in reach.
/// 2. Spawns the three themed buttons — Execute / Stabilize / Open Door — each carrying its
///    marker and each [`Visibility::Hidden`](bevy::render::view::Visibility). The live
///    `detect_contextual_targets` system reveals Execute / Stabilize each only when its
///    [`ContextualTargets`](super::super::components::ContextualTargets) field is `Some`; Open
///    Door stays hidden (no sim verb yet — see the in-line note).
/// 3. Parents the three buttons under the panel box (the box is the root).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawns + the theme read.
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
    // `detect_contextual_targets` reveals it when a downed neighbour is in reach.
    let panel = spawn_panel(&mut commands, &theme);
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.right = Val::Vw(CONTEXTUAL_PANEL_RIGHT_VW);
            node.bottom = Val::Vh(CONTEXTUAL_PANEL_BOTTOM_VH);
            node.width = Val::Vw(CONTEXTUAL_PANEL_WIDTH_VW);
            // FIT the stacked buttons vertically — the panel is exactly as tall as its three
            // buttons + the inter-button gaps.
            node.height = Val::Auto;
            // Stack the three buttons in a column with a responsive inter-button gap.
            node.flex_direction = FlexDirection::Column;
            node.row_gap = Val::Vh(CONTEXTUAL_PANEL_ROW_GAP_VH);
        });
    commands.entity(panel).insert((
        ContextualPanelRoot,
        // Hidden by default — `detect_contextual_targets` reveals the panel only when a downed
        // neighbour is in reach (`ui-mutate-not-respawn`: it toggles Visibility in place).
        Visibility::Hidden,
        // Stack the whole panel subtree ABOVE the opaque bottom bar (GlobalZIndex 10) it overlaps,
        // and above the on-bar weapon + stance cluster (GlobalZIndex 11). Without this the panel
        // resolved the right camera + position + size yet rendered nothing — the bar painted over
        // it (the GTW-294 occlusion bug). GlobalZIndex propagates to the button children.
        GlobalZIndex(CONTEXTUAL_PANEL_Z),
    ));

    // The three contextual buttons — each carries its marker and is hidden by default;
    // `detect_contextual_targets` reveals Execute / Stabilize when its `ContextualTargets` field
    // is `Some` (Open Door stays hidden — see the in-line note below).
    let execute = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Execute"),
        (ExecuteButton, Visibility::Hidden),
    );
    let stabilize = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Stabilize"),
        (StabilizeButton, Visibility::Hidden),
    );
    // The Open-Door button is spawned but DELIBERATELY stays hidden and inert: the Open-Door
    // act has no sim verb yet because the model has no door / interactable-object concept (the
    // sim's `downed_acts` ships only Execute + Stabilize). `detect_contextual_targets` therefore
    // never offers an Open-Door target, so this button is never revealed and never routes an
    // intent. Delivering the door/interactable model + its act is out of scope for GTW-294.
    // TODO(GTW-315): Open-Door act needs door/interactable objects; button stays hidden until then.
    let open_door = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Open Door"),
        (OpenDoorButton, Visibility::Hidden),
    );

    // Parent the three buttons under the panel BOX (the root) — they inherit its resolved
    // UI-camera target and its `GlobalZIndex`, so the whole subtree draws above the bottom bar.
    commands
        .entity(panel)
        .add_children(&[execute, stabilize, open_door]);
}

/// Despawns the contextual panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`ContextualPanelRoot`] entity (and so its three button children) so
/// the whole subtree is gone the moment the battle leaves `BattleRunning` — battle-scoped
/// lifecycle. The battlescape neighborhood cleans up explicitly on `OnExit` rather than via
/// `DespawnOnExit` markers, so this mirrors that. Param-only (`bevy-traps.md` #7): [`Commands`] +
/// a `Query<Entity, With<ContextualPanelRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_contextual_panel(
    mut commands: Commands,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

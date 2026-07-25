//! The `bevy_ui` rendering of the comparison panel, and its spawn / despawn lifecycle.
//!
//! Deliberately un-themed (no [`gdtf_ui`] widget builder, no theme read), for the same
//! reason the GTW-819 spike's button was: the fewer moving parts between the harness state
//! and the pixels, the sharper the comparison. An absolutely-positioned overlay with a high
//! [`GlobalZIndex`] so it is visible over whatever screen `AppState::Running` is showing.

use bevy::{
    color::palettes::css::{BLACK, LIME},
    prelude::*,
    ui::{BackgroundColor, GlobalZIndex, Node, PositionType, Val},
};

use super::{
    latch::PendingUiStackSwap,
    stack::{UiStack, UiStackId},
};
use crate::states::AppState;

/// The harness overlay's stacking order — above every in-game HUD band (the battlescape's
/// highest is the contextual panel at `20`) so the comparison panel is never occluded by
/// the screen it is overlaid on. Framework plumbing fed straight to [`GlobalZIndex`].
const SWAP_PANEL_Z: i32 = 910;

crate::support_item! {
    /// Marks the root of the `bevy_ui` rendering of the comparison panel.
    ///
    /// The ONE handle the no-leak assertion counts: when the egui stack is live there must
    /// be zero entities carrying this marker.
    #[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
    struct UiSwapBevyUiPanel;
}

crate::support_item! {
    /// Marks the `bevy_ui` rendering's swap button — the entity whose [`Interaction`] a
    /// click (real or injected) drives.
    #[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
    struct UiSwapBevyUiButton;
}

crate::support_item! {
    /// The caption both renderings put on their swap control, so the two screens say the
    /// same thing and a screenshot comparison is not confounded by different wording.
    const UI_SWAP_CAPTION: &str = "swap UI stack (F9)";
}

/// Spawn the `bevy_ui` rendering.
///
/// Scoped with [`DespawnOnExit`] so it leaves with `AppState::Running` — the despawn-on-exit
/// half of the lifecycle, which no swap can defeat, because a state exit takes the entity
/// with it whether or not the panel happened to be the live stack at the time.
fn spawn_panel(commands: &mut Commands) {
    commands
        .spawn((
            UiSwapBevyUiPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(120.0),
                width: Val::Px(280.0),
                height: Val::Px(56.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(SWAP_PANEL_Z),
            DespawnOnExit(AppState::Running),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Button,
                    UiSwapBevyUiButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(LIME.into()),
                ))
                .with_child((Text::new(UI_SWAP_CAPTION), TextColor(BLACK.into())));
        });
}

/// `OnEnter(AppState::Running)`: spawn the `bevy_ui` rendering if it is the live stack.
///
/// The spawn-on-enter half of the lifecycle. Reads the harness state as `Option`
/// (`bevy-traps.md` #1) so it is inert where the harness is absent. Param-only
/// (`bevy-traps.md` #7): one optional resource plus [`Commands`].
pub(super) fn spawn_bevy_ui_panel_on_enter(stack: Option<Res<UiStack>>, mut commands: Commands) {
    if stack.is_some_and(|stack| stack.is_live(UiStackId::BevyUi)) {
        spawn_panel(&mut commands);
    }
}

/// `Update`: make the `bevy_ui` rendering's presence match the live stack.
///
/// Spawn-on-becoming-live / despawn-on-becoming-inactive — the swap half of the lifecycle,
/// and the guarantee behind "the inactive stack leaks no entities": the moment the egui
/// stack is live, every entity of this one is despawned (recursively, so the button and its
/// caption go with the root).
///
/// Written as a CONVERGENCE (compare desired presence against actual presence) rather than
/// a reaction to a change flag, so it is idempotent: a second run in the same state does
/// nothing, and no missed change event can leave the screen showing two stacks at once.
///
/// Param-only (`bevy-traps.md` #7): one optional resource, one filtered query, [`Commands`].
pub(super) fn sync_bevy_ui_panel(
    stack: Option<Res<UiStack>>,
    panels: Query<Entity, With<UiSwapBevyUiPanel>>,
    mut commands: Commands,
) {
    let Some(stack) = stack else {
        return;
    };
    let wanted = stack.is_live(UiStackId::BevyUi);
    let mut present = false;
    for panel in &panels {
        if wanted {
            present = true;
        } else {
            commands.entity(panel).despawn();
        }
    }
    if wanted && !present {
        spawn_panel(&mut commands);
    }
}

/// `Update`: latch a swap when the `bevy_ui` rendering's swap button is pressed.
///
/// Reads the `Changed<Interaction>` edge `bevy_ui`'s own `ui_focus_system` writes for a real
/// click (`bevy-traps.md` #6) and writes the SAME [`PendingUiStackSwap`] latch the keyboard
/// and the wire write — one funnel, so this button and its egui twin land in the same state.
pub(super) fn record_bevy_ui_swap_press(
    pressed: Query<&Interaction, (Changed<Interaction>, With<UiSwapBevyUiButton>)>,
    mut commands: Commands,
) {
    for interaction in &pressed {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(PendingUiStackSwap::toggle());
        }
    }
}

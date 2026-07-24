//! The `bevy_ui` half of the coexistence spike: ONE hand-rolled [`Button`] and the system
//! that records its presses.
//!
//! Deliberately un-themed (no [`gdtf_ui`] widget builder, no
//! [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) read): the spike answers a rendering / input
//! question about two UI STACKS, so the fewer moving parts between the click and the tally,
//! the sharper the answer. The button is an absolutely-positioned overlay with a high
//! [`GlobalZIndex`] so it is visible on whatever screen `AppState::Running` is showing.

use bevy::{
    color::palettes::css::{BLACK, LIME},
    prelude::*,
    ui::{BackgroundColor, GlobalZIndex, Node, PositionType, Val},
};

use super::counters::{ClickCount, UiStackClicks};

/// The spike overlay's stacking order — above every in-game HUD band (the battlescape's
/// highest is the contextual panel at `20`) so the spike is never occluded by the screen it
/// is overlaid on. Framework plumbing fed straight to [`GlobalZIndex`], not a domain value.
const SPIKE_Z: i32 = 900;

crate::support_item! {
    /// Marks the spike's `bevy_ui` button, so the coexistence test can find the one entity
    /// whose [`Interaction`] it drives.
    #[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
    struct CoexistenceBevyUiButton;
}

/// Marks the caption inside the spike's `bevy_ui` button, so the press system can rewrite it
/// with the live tally (the on-screen half of the evidence).
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub(super) struct CoexistenceBevyUiLabel;

/// The caption text for a given tally — one place, so the spawn and the update agree.
pub(super) fn bevy_ui_caption(clicks: ClickCount) -> String {
    format!("bevy_ui button: {}", clicks.get())
}

/// Spawns the spike's `bevy_ui` button on `OnEnter(AppState::Running)`.
///
/// Scoped with [`DespawnOnExit`] so it leaves with the `Running` phase — the spike never
/// outlives the state it was added to. Param-only (`bevy-traps.md` #7): [`Commands`] only.
pub(super) fn spawn_coexistence_bevy_ui_button(mut commands: Commands) {
    commands
        .spawn((
            Button,
            CoexistenceBevyUiButton,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(220.0),
                width: Val::Px(260.0),
                height: Val::Px(56.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(LIME.into()),
            GlobalZIndex(SPIKE_Z),
            DespawnOnExit(crate::states::AppState::Running),
        ))
        .with_child((
            Text::new(bevy_ui_caption(ClickCount::default())),
            TextColor(BLACK.into()),
            CoexistenceBevyUiLabel,
        ));
}

/// Records a press of the spike's `bevy_ui` button and rewrites its caption.
///
/// Reads `Changed<Interaction>` — the edge `bevy_ui`'s own `ui_focus_system` writes for a real
/// click (`bevy-traps.md` #6), which is also what the headless test drives. Bumping ONLY
/// [`UiStackClicks::bump_bevy_ui`] is half the coexistence proof: an egui click must leave this
/// tally untouched, and vice versa.
///
/// Param-only (`bevy-traps.md` #7): two queries and a `ResMut`.
pub(super) fn record_bevy_ui_button_press(
    pressed: Query<&Interaction, (Changed<Interaction>, With<CoexistenceBevyUiButton>)>,
    mut labels: Query<&mut Text, With<CoexistenceBevyUiLabel>>,
    mut clicks: ResMut<UiStackClicks>,
) {
    for interaction in &pressed {
        if *interaction != Interaction::Pressed {
            continue;
        }
        clicks.bump_bevy_ui();
        let caption = bevy_ui_caption(clicks.bevy_ui());
        info!("ui-coexistence: BEVY_UI button pressed ({caption})");
        for mut text in &mut labels {
            (**text).clone_from(&caption);
        }
    }
}

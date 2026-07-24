//! The egui half of the coexistence spike: ONE egui button drawn beside the `bevy_ui` one,
//! plus the latch that turns its click into a tally bump exactly once.
//!
//! Runs in [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass), never `Update`
//! (`bevy-traps.md` #8), and takes `ctx_mut()` as a `Result` — never unwrapped.
//!
//! # Why the click LATCHES instead of counting in place
//!
//! Multipass runs the egui closure up to TWICE per frame (`bevy-traps.md` #8b), and both
//! passes see the same pointer input — so `if ui.button(..).clicked() { count += 1 }` would
//! record ONE user click as TWO. The closure therefore only ever performs an IDEMPOTENT act
//! (inserting the fieldless [`EguiClickPending`] resource — a second insert is the same
//! world), and [`apply_egui_click`] (an ordinary `Update` system) converts a pending latch
//! into exactly one bump and removes it. Same shape as the procgen stepper's
//! command latch, for the same reason.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::counters::{ClickCount, UiStackClicks};

crate::support_item! {
    /// Where the egui panel is pinned, in egui points from the window's top-left.
    ///
    /// Fixed (not draggable / not auto-placed), and the BUTTON is the panel's first widget, so
    /// the button's own top-left is this position: the panel sits BESIDE the `bevy_ui` button
    /// rather than on top of it (an overlap would make "which stack got the click"
    /// unanswerable by eye), and the coexistence test can aim a pointer at the egui button
    /// without guessing at egui's frame margins — which is why it widens to `pub` under
    /// `test-support`.
    const EGUI_PANEL_POS: egui::Pos2 = egui::pos2(320.0, 220.0);
}

/// Marks that the egui button was clicked and the tally has not been bumped yet.
///
/// A fieldless marker RESOURCE rather than a boolean field: inserting it is idempotent, which
/// is precisely the property the multipass closure needs (see the module doc).
#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub(super) struct EguiClickPending;

/// The egui button's caption for a given tally — one place, so the label and any assertion
/// about it agree.
pub(super) fn egui_caption(clicks: ClickCount) -> String {
    format!("egui button: {}", clicks.get())
}

/// Draws the spike's egui panel and latches a click.
///
/// Returns `Result` so a missing primary egui context (`ctx_mut()?`) is handled rather than
/// unwrapped (`bevy-traps.md` #8). Takes the tallies as `Option<Res<…>>` so it is inert if the
/// resource is somehow absent (`bevy-traps.md` #1).
pub(super) fn draw_coexistence_egui_panel(
    mut contexts: EguiContexts,
    clicks: Option<Res<UiStackClicks>>,
    mut commands: Commands,
) -> Result {
    let Some(clicks) = clicks else {
        return Ok(());
    };
    let caption = egui_caption(clicks.egui());
    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("gdtf-ui-coexistence-spike"))
        .fixed_pos(EGUI_PANEL_POS)
        .show(ctx, |ui| {
            // The button FIRST, so its top-left is exactly `EGUI_PANEL_POS` (no frame margin
            // between the pinned position and the clickable rect).
            if ui.button(caption).clicked() {
                // Idempotent under a multipass re-run — see the module doc.
                commands.insert_resource(EguiClickPending);
            }
            ui.label("egui stack (GTW-819)");
        });
    Ok(())
}

/// Converts a latched egui click into exactly one tally bump.
///
/// Runs in `Update` (the latch is written during the egui pass in `PostUpdate`, so it is
/// applied on the following frame — the spike's tallies are eventually-consistent by one
/// frame, which no assertion here depends on being tighter).
///
/// Param-only (`bevy-traps.md` #7): an `Option<Res<…>>` latch, a `ResMut` tally, `Commands`.
pub(super) fn apply_egui_click(
    pending: Option<Res<EguiClickPending>>,
    mut clicks: ResMut<UiStackClicks>,
    mut commands: Commands,
) {
    if pending.is_none() {
        return;
    }
    commands.remove_resource::<EguiClickPending>();
    clicks.bump_egui();
    info!(
        "ui-coexistence: EGUI button pressed ({})",
        egui_caption(clicks.egui())
    );
}

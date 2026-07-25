//! The egui rendering of the comparison panel — the same one control, drawn through the
//! other stack.
//!
//! Runs in [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass), never `Update`
//! (`bevy-traps.md` #8), and takes `ctx_mut()` as a `Result` — never unwrapped.
//!
//! # The multipass double-swap this file must not commit
//!
//! egui's multipass runs this closure up to TWICE per frame, and both passes see the same
//! pointer input (`bevy-traps.md` #8b). A swap performed inside the closure would therefore
//! run TWICE for one user click — flipping to the other stack and straight back, so the
//! swap would appear not to work at all. The closure consequently performs only the
//! IDEMPOTENT act of writing the single-slot [`PendingUiStackSwap`] latch (a second write
//! in the same frame overwrites the first slot, leaving the same world), and
//! [`apply_ui_stack_swap`](super::latch::apply_ui_stack_swap) — an ordinary `Update` system
//! — turns that slot into exactly one swap.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::{
    bevy_ui_panel::UI_SWAP_CAPTION,
    latch::PendingUiStackSwap,
    stack::{UiStack, UiStackId},
};

crate::support_item! {
    /// Where the egui rendering is pinned, in egui points from the window's top-left.
    ///
    /// The SAME screen position the `bevy_ui` rendering occupies, so swapping stacks moves
    /// nothing on screen — the comparison is between two drawings of one control, not
    /// between two layouts. Fixed (not draggable, not auto-placed) and the button is the
    /// area's first widget, so the button's own top-left is this position: a headless test
    /// can aim a synthetic pointer at it without guessing at egui's frame margins, which is
    /// why it widens to `pub` under `test-support`.
    const EGUI_SWAP_PANEL_POS: egui::Pos2 = egui::pos2(24.0, 120.0);
}

/// [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass): draw the egui rendering
/// while it is the live stack, and latch a click on its swap button.
///
/// Draws NOTHING at all when the other stack is live — the egui half of "the inactive stack
/// is not on screen", the counterpart of the `bevy_ui` half's despawn.
///
/// Returns `Result` so a missing primary egui context (`ctx_mut()?`) is handled rather than
/// unwrapped (`bevy-traps.md` #8). Takes the harness state as `Option<Res<…>>` so it is
/// inert if the resource is absent (`bevy-traps.md` #1).
pub(super) fn draw_ui_swap_egui_panel(
    mut contexts: EguiContexts,
    stack: Option<Res<UiStack>>,
    mut commands: Commands,
) -> Result {
    let Some(stack) = stack else {
        return Ok(());
    };
    if !stack.is_live(UiStackId::Egui) {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("gdtf-ui-swap-harness"))
        .fixed_pos(EGUI_SWAP_PANEL_POS)
        .show(ctx, |ui| {
            // The button FIRST, so its top-left is exactly `EGUI_SWAP_PANEL_POS`.
            if ui.button(UI_SWAP_CAPTION).clicked() {
                // Idempotent under a multipass re-run — see the module doc.
                commands.insert_resource(PendingUiStackSwap::toggle());
            }
        });
    Ok(())
}

//! The harness's keyboard shortcut — the developer-facing half of the swap.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

use super::latch::PendingUiStackSwap;

crate::support_item! {
    /// The key that swaps the live UI stack.
    ///
    /// `F9`, chosen because the function row is where this repo already puts its DEV-only,
    /// non-diegetic affordances (`gdtf_screenshot`'s `F10` capture keybind) and because
    /// nothing — not the authored `keybinds.tuning.ron` act table, not the editor hotkeys,
    /// not `F10` — currently claims it, so the swap can never be confused with a game act.
    /// A fixed const rather than an entry in the authored keybind table on purpose: the
    /// table is the PLAYER's act bindings, and this affordance does not exist in a build a
    /// player runs.
    const UI_STACK_SWAP_KEY: KeyCode = KeyCode::F9;
}

/// `Update`: latch a swap when the shortcut is tapped.
///
/// Writes the SAME [`PendingUiStackSwap`] latch the wire intent and the on-screen swap
/// buttons write, so the keyboard is not a second implementation of the swap — it is one
/// more caller of the one funnel.
///
/// Two guards are applied where this system is REGISTERED (see
/// [`super::plugin`]), not here, so the run condition is visible next to the wiring:
/// `not(egui_wants_any_keyboard_input)` — an egui widget with keyboard focus owns the
/// keystroke (`bevy-traps.md` #8) — and `playback_caught_up`, the sim's own global input
/// gate, so the shortcut cannot fire on a frame where the player's input is being withheld
/// while the presenter catches up with the sim.
///
/// Param-only (`bevy-traps.md` #7): the buffered key state plus [`Commands`].
pub(super) fn ui_stack_swap_key(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keys.just_pressed(UI_STACK_SWAP_KEY) {
        commands.insert_resource(PendingUiStackSwap::toggle());
    }
}

/// Whether an egui widget currently wants the keystroke — the run condition that keeps the
/// shortcut from stealing typing out of an egui text field (`bevy-traps.md` #8).
///
/// Reads the SAME shipped [`EguiWantsInput`] state `bevy_egui`'s own
/// `egui_wants_any_keyboard_input` reads, but through `Option<Res<…>>` rather than `Res<…>`
/// (`bevy-traps.md` #1): the harness registers the shortcut in every build, including the
/// renderer-less headless ones where the egui half is skipped and the resource therefore
/// never exists, and `bevy_egui`'s own form would panic there instead of reporting "no egui
/// wants this key".
pub(super) fn egui_holds_the_keyboard(wants: Option<Res<EguiWantsInput>>) -> bool {
    wants.is_some_and(|wants| wants.wants_any_keyboard_input())
}

//! [`UiSwapHarnessPlugin`] — the DEV UI-stack swap harness's wiring (GTW-816).
//!
//! ## Which state, and why
//!
//! [`AppState::Running`] — the state that owns the persistent
//! [`UiCamera`](crate::states::running::UiCamera) both stacks render through, and the state
//! that spans every screen the GTW-796 comparison children will plug into (the main menu,
//! the battlescape HUD, the editor host). Attaching the harness here means the swap works
//! on whichever of those screens the app is showing, rather than only inside one of them.
//!
//! ## What is registered
//!
//! - `OnEnter(AppState::Running)`:
//!   [`spawn_bevy_ui_panel_on_enter`] — the spawn-on-enter half of the lifecycle; the
//!   despawn-on-exit half rides the panel's own `DespawnOnExit(AppState::Running)`.
//! - `Update`, gated `in_state(Running)` and CHAINED so a swap is requested, applied, and
//!   reflected on screen within one frame: [`ui_stack_swap_key`] (the keyboard trigger),
//!   [`record_bevy_ui_swap_press`] (the on-screen trigger),
//!   [`apply_ui_stack_swap`] (the ONE writer of the live stack), then
//!   [`sync_bevy_ui_panel`] (spawn / despawn to match).
//! - `Update`, gated `in_state(Running)`: [`bind_primary_egui_context`].
//! - [`EguiPrimaryContextPass`], gated `in_state(Running)`: [`draw_ui_swap_egui_panel`] —
//!   NEVER `Update` (`bevy-traps.md` #8).
//!
//! ## The two guards on the keyboard shortcut
//!
//! [`ui_stack_swap_key`] alone carries two run conditions, both of them about NOT stealing
//! a keystroke that belongs to something else:
//!
//! - `not(egui_holds_the_keyboard)` — while an egui widget holds keyboard focus the
//!   keystroke is that widget's (`bevy-traps.md` #8). It reads the same shipped
//!   `EguiWantsInput` state the content editor's own hotkey guard reads, through an
//!   `Option<Res<…>>` so it is inert rather than panicking where egui is absent.
//! - `playback_caught_up` — the sim's own global input gate. While the presenter is still
//!   replaying what the sim has already done, the player's input is withheld; the swap
//!   shortcut respects the same gate, so it cannot fire on a frame where the sim is
//!   consuming the keypress stream on the player's behalf.
//!
//! ## `EguiPlugin` ownership is ONE decision, and it is made here
//!
//! Bevy panics on a duplicate plugin add, and three `dev_tools` affordances want egui (this
//! harness, the procgen stepper, the GTW-819 coexistence proof-of-concept). This plugin is
//! added FIRST by the dev aggregate and makes the decision for all of them: it adds
//! [`EguiPlugin`], turns OFF `auto_create_primary_context`, and binds the context to the
//! game's UI camera itself. The other two guard with [`App::is_plugin_added`] and therefore
//! defer to this one whenever it is present.
//!
//! The one condition on that decision is the RENDER stack: `EguiPlugin` reads
//! `Assets<Shader>`, which only exists once Bevy's [`RenderPlugin`] is in the app, so a
//! `MinimalPlugins` harness panics on the very first frame if egui is added there. The
//! harness therefore adds the egui half only where the render stack exists. A real binary
//! always has it, so shipped wiring is unaffected; a `MinimalPlugins` integration test gets
//! the `bevy_ui` stack, the swap state, the keyboard path and the wire path, with the egui
//! DRAW simply absent (which is exactly what a windowless, renderer-less app can show).

use bevy::{prelude::*, render::RenderPlugin};
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};
use gdtf_battle_presenter::playback_caught_up;

use super::{
    bevy_ui_panel::{record_bevy_ui_swap_press, spawn_bevy_ui_panel_on_enter, sync_bevy_ui_panel},
    egui_context::bind_primary_egui_context,
    egui_panel::draw_ui_swap_egui_panel,
    keyboard::{egui_holds_the_keyboard, ui_stack_swap_key},
    latch::apply_ui_stack_swap,
    stack::UiStack,
};
use crate::states::AppState;

crate::support_item! {
    /// The DEV UI-stack swap harness (GTW-816).
    ///
    /// Compiled only under the `dev_tools` feature (which is what pulls `bevy_egui` in at
    /// all), and added only through the crate-private `DevAffordancesPlugin` aggregate — a
    /// release artifact never sees it. (Named, not linked: the aggregate is `pub(crate)`,
    /// and a link from this `test-support`-public item would be a `private_intra_doc_links`
    /// denial.)
    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
    struct UiSwapHarnessPlugin;
}

impl Plugin for UiSwapHarnessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiStack>();
        app.add_systems(OnEnter(AppState::Running), spawn_bevy_ui_panel_on_enter);
        app.add_systems(
            Update,
            (
                ui_stack_swap_key
                    .run_if(not(egui_holds_the_keyboard))
                    .run_if(playback_caught_up),
                record_bevy_ui_swap_press,
                apply_ui_stack_swap,
                sync_bevy_ui_panel,
            )
                .chain()
                .run_if(in_state(AppState::Running)),
        );
        // The egui half needs the render stack (see the module doc); everything above works
        // without it.
        if !app.is_plugin_added::<RenderPlugin>() {
            return;
        }
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
            // Take control of WHERE the primary egui context lives, rather than letting it
            // land on whichever camera spawns first (see `super::egui_context`).
            app.insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            });
        }
        app.add_systems(
            Update,
            bind_primary_egui_context.run_if(in_state(AppState::Running)),
        );
        app.add_systems(
            EguiPrimaryContextPass,
            draw_ui_swap_egui_panel.run_if(in_state(AppState::Running)),
        );
    }
}

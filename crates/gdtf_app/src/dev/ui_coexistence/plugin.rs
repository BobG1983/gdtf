//! [`UiCoexistencePlugin`] — the GTW-819 proof-of-concept wiring: ONE `bevy_ui` button and
//! ONE egui button, alive at the same time, in the same [`AppState`].
//!
//! ## Which state, and why
//!
//! [`AppState::Running`] — the cheapest state that actually has a UI to coexist with. The
//! machine walks `Init → Load → Intro → Running` on its own, so `Running` is reached by
//! launching the app and waiting (no battle, no situation, no procgen), and it is where the
//! persistent [`UiCamera`](crate::states::running::UiCamera) lives — the camera `bevy_ui`
//! renders through and the one the DEV `net_qa` capture path retargets offscreen. Any deeper
//! state (a battle) would add generation time and a pile of unrelated moving parts to a spike
//! whose whole point is to isolate ONE question.
//!
//! ## What is registered
//!
//! - `OnEnter(AppState::Running)`: the `bevy_ui` button
//!   ([`spawn_coexistence_bevy_ui_button`]), scoped to leave with the state.
//! - `Update`, gated `in_state(Running)`: [`record_bevy_ui_button_press`] (the `bevy_ui`
//!   activation edge), [`apply_egui_click`] (the latched egui activation), and
//!   [`bind_primary_egui_context`] (pins egui to the UI camera).
//! - [`EguiPrimaryContextPass`], gated `in_state(Running)`:
//!   [`draw_coexistence_egui_panel`] — NEVER `Update` (`bevy-traps.md` #8).
//!
//! ## Sharing `EguiPlugin` with the other dev affordance
//!
//! The procgen stepper (`crate::dev::procgen_stepper`) also adds
//! [`EguiPlugin`](bevy_egui::EguiPlugin) when its own env gate is on, and Bevy panics on a
//! duplicate plugin add. So this plugin adds egui only if nothing else did
//! ([`App::is_plugin_added`]) — a constraint worth carrying forward: with two UI stacks in one
//! build, `EguiPlugin` ownership has to be a single decision, not a per-feature add.

use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};

use super::{
    bevy_ui_button::{record_bevy_ui_button_press, spawn_coexistence_bevy_ui_button},
    counters::UiStackClicks,
    egui_button::{apply_egui_click, draw_coexistence_egui_panel},
    egui_context::bind_primary_egui_context,
};
use crate::states::AppState;

crate::support_item! {
    /// The DEV-ONLY UI-stack coexistence proof-of-concept (GTW-819).
    ///
    /// Compiled only under the `dev_tools` feature (which is what pulls `bevy_egui` in at
    /// all), and added only through the crate-private `DevAffordancesPlugin` aggregate — a
    /// release artifact never sees it. (Named, not linked: the aggregate is `pub(crate)`, and a
    /// link from this `test-support`-public item would be a `private_intra_doc_links` denial.)
    #[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
    struct UiCoexistencePlugin;
}

impl Plugin for UiCoexistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiStackClicks>();
        // Another dev affordance may already own the egui wiring (see the module doc).
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
            // Take control of WHERE the primary egui context lives, rather than letting it
            // land on whichever camera spawns first (see `super::egui_context`).
            app.insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            });
        }
        app.add_systems(OnEnter(AppState::Running), spawn_coexistence_bevy_ui_button);
        app.add_systems(
            Update,
            (
                bind_primary_egui_context,
                record_bevy_ui_button_press,
                apply_egui_click,
            )
                .run_if(in_state(AppState::Running)),
        );
        app.add_systems(
            EguiPrimaryContextPass,
            draw_coexistence_egui_panel.run_if(in_state(AppState::Running)),
        );
    }
}

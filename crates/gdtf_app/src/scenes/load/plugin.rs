use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_sim::situation::Situation;
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::{
    scenes::load::{resources::LoadHandles, systems::*},
    states::AppState,
};

pub(in crate::scenes) struct LoadScenePlugin;

impl Plugin for LoadScenePlugin {
    fn build(&self, app: &mut App) {
        // Register the generic RON loader for the theme spec once (GTW-136):
        // this installs `Assets<RonAsset<GdtfThemeSpec>>` and its loader so the
        // kick-off can `asset_server.load::<RonAsset<GdtfThemeSpec>>(..)`.
        //
        // `init_asset` immediately requires the `AssetServer` resource, so it
        // panics under `MinimalPlugins` (no `AssetPlugin`). Guard on the server's
        // presence: in the production app and the real-asset harness the server
        // exists and the loader registers; in the `MinimalPlugins` state-machine
        // harness it is absent and `kick_off_loads` already no-ops, so skipping
        // the registration there is correct (bevy-traps rule 1).
        //
        // GTW-205 (E10.3): the authored `Situation` loads through the SAME generic
        // RON loader, so register `Assets<RonAsset<Situation>>` + its loader behind
        // the same `asset_server.is_some()` guard as the theme.
        if app.world().get_resource::<AssetServer>().is_some() {
            app.init_ron_asset::<GdtfThemeSpec>();
            app.init_ron_asset::<Situation>();
        }
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(
        OnEnter(AppState::Load),
        (print_on_enter, kick_off_loads).chain(),
    )
    .add_systems(
        Update,
        (
            // poll/resolve runs until a GdtfTheme is inserted (success path
            // resolves the loaded spec; failure path inserts the const
            // default). Ordered BEFORE the transition so the theme is present
            // when the transition checks for it (bevy-traps rule 3).
            poll_and_resolve.run_if(
                in_state(AppState::Load)
                    .and(resource_exists::<LoadHandles>)
                    .and(not(resource_exists::<GdtfTheme>)),
            ),
            // Once a GdtfTheme exists, leave Load for Intro.
            transition_to_intro.run_if(in_state(AppState::Load).and(resource_exists::<GdtfTheme>)),
        )
            .chain(),
    )
    .add_systems(OnExit(AppState::Load), (print_on_exit, cleanup).chain());
}

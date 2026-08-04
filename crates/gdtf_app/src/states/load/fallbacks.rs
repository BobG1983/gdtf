//! Headless load fallbacks for tests without an asset server.

#[cfg(feature = "headless_test")]
use bevy::prelude::*;
#[cfg(feature = "headless_test")]
use gdtf_battle_sim::{
    injuries::{InjuryRegistry, InjuryTables},
    level::PrefabRegistry,
    procgen::ProcgenTuning,
    tuning::CombatTuning,
};
#[cfg(feature = "headless_test")]
use gdtf_ui::theme::default_theme;

#[cfg(feature = "headless_test")]
use crate::states::load::resources::LoadedSituation;

/// Insert default load resources for headless tests when assets are absent.
#[cfg(feature = "headless_test")]
pub fn seed_load_fallbacks(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
    commands.insert_resource(default_theme());
    commands.insert_resource(CombatTuning::default());
    if asset_server.is_none() {
        commands.insert_resource(InjuryRegistry::default());
        commands.insert_resource(InjuryTables::default());
        commands.insert_resource(PrefabRegistry::default());
        commands.insert_resource(ProcgenTuning::default());
        commands.insert_resource(LoadedSituation::new(
            gdtf_battle_sim::situation::Situation::default(),
        ));
    }
}

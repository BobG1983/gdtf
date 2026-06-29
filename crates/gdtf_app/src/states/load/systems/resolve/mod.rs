//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.
//!
//! The orchestrator [`poll_and_resolve`] drives one poll per frame; each per-asset
//! resolve concern lives in its own sibling module: the theme (inline in the
//! orchestrator + its `fall_back` helper, in `poll`), the tuning, the situation, the
//! weapons registry, the armor registry (GTW-269), and the terrain registry (GTW-394).
//! The `params` module holds the two `SystemParam` bundles the orchestrator reads.

pub(in crate::states::load) mod armor;
pub(in crate::states::load) mod gangs;
pub(in crate::states::load) mod injuries;
mod params;
mod poll;
pub(in crate::states::load) mod prefab_v2;
pub(in crate::states::load) mod prefabs;
mod situation;
pub(in crate::states::load) mod stat_tuning;
pub(in crate::states::load) mod terrain;
pub(in crate::states::load) mod terrain_model;
pub(in crate::states::load) mod themes;
pub(in crate::states::load) mod tuning;
pub(in crate::states::load) mod weapons;

pub(in crate::states::load) use armor::redrive_armor_on_asset_event;
pub(in crate::states::load) use gangs::redrive_gangs_on_asset_event;
pub(in crate::states::load) use injuries::redrive_injuries_on_asset_event;
pub(in crate::states::load) use poll::poll_and_resolve;
pub(in crate::states::load) use prefab_v2::redrive_prefabs_v2_on_asset_event;
pub(in crate::states::load) use prefabs::redrive_prefabs_on_asset_event;
pub(in crate::states::load) use stat_tuning::redrive_stat_tuning_on_asset_event;
pub(in crate::states::load) use terrain::redrive_terrain_on_asset_event;
pub(in crate::states::load) use terrain_model::{
    redrive_terrain_defs_on_asset_event, redrive_theme_defs_on_asset_event,
};
pub(in crate::states::load) use themes::redrive_themes_on_asset_event;
pub(in crate::states::load) use tuning::redrive_combat_tuning_on_asset_event;
pub(in crate::states::load) use weapons::redrive_weapons_on_asset_event;

#[cfg(test)]
pub(super) mod hot_reload_test_support;

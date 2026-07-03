//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.
//!
//! The orchestrator [`poll_and_resolve`] drives one poll per frame; each per-asset
//! FOLDER-resolve concern lives in its own sibling module: the theme (inline in the
//! orchestrator + its `fall_back` helper, in `poll`), the weapons registry, the
//! armor registry (GTW-269), the UUID-keyed terrain + theme defs (GTW-487), and the
//! UUID-keyed prefabs (GTW-489). The `params` module holds the two `SystemParam`
//! bundles the orchestrator reads. GTW-564 moved the four SINGLE-FILE chains
//! (situation + combat / stat / procgen tuning) onto the generic hot-RON seam —
//! their resolve/redrive modules are gone; the Load plugin registers each with one
//! ext call.
//!
//! GTW-494 (child T08 of GTW-476) RETIRED the old flat-dir `terrain` / `themes` /
//! `prefabs` resolvers — the UUID-model `terrain_model` + `prefab` loaders are now
//! the only terrain / theme / prefab resolvers (the sim + procgen + presenter consume
//! the new registries as of GTW-491/492/493). GTW-557 dropped the `_v2` suffix from
//! the prefab module name and all its exported symbols.

pub(in crate::states::load) mod armor;
pub(in crate::states::load) mod attachments;
pub(in crate::states::load) mod fields;
pub(in crate::states::load) mod gangs;
pub(in crate::states::load) mod injuries;
pub(in crate::states::load) mod melee_weapons;
mod params;
mod poll;
pub(in crate::states::load) mod prefab;
pub(in crate::states::load) mod terrain_model;
pub(in crate::states::load) mod weapons;

pub(in crate::states::load) use armor::redrive_armor_on_asset_event;
pub(in crate::states::load) use attachments::redrive_attachments_on_asset_event;
pub(in crate::states::load) use fields::redrive_fields_on_asset_event;
pub(in crate::states::load) use gangs::redrive_gangs_on_asset_event;
pub(in crate::states::load) use injuries::redrive_injuries_on_asset_event;
pub(in crate::states::load) use melee_weapons::redrive_melee_weapons_on_asset_event;
pub(in crate::states::load) use poll::poll_and_resolve;
pub(in crate::states::load) use prefab::redrive_prefabs_on_asset_event;
pub(in crate::states::load) use terrain_model::{
    redrive_terrain_defs_on_asset_event, redrive_theme_defs_on_asset_event,
};
pub(in crate::states::load) use weapons::redrive_weapons_on_asset_event;

#[cfg(test)]
pub(super) mod hot_reload_test_support;

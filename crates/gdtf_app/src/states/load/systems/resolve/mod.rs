//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.
//!
//! The orchestrator [`poll_and_resolve`] drives one poll per frame; each BESPOKE
//! per-asset FOLDER-resolve concern lives in its own sibling module: the theme
//! (inline in the orchestrator + its `fall_back` helper, in `poll`), the
//! injuries registry + tables (GTW-437, one folder → two resources), and the
//! UUID-keyed prefab multimap (GTW-489). The `params` module holds the
//! `SystemParam` bundles the orchestrator reads.
//!
//! GTW-564 moved the four SINGLE-FILE chains (situation + combat / stat /
//! procgen tuning) onto the generic hot-RON seam, GTW-570 moved the seven
//! FOLDER content families (ranged/melee weapons, armor, fields, gangs, terrain
//! and theme defs) onto the generic content-family seam, and GTW-619 moved the
//! attachments folder (GTW-549) onto the same seam — their resolve/redrive
//! modules are gone; the Load plugin registers each with one ext call. The
//! two modules here are the DECLARED GTW-570 exclusions, bespoke by design
//! (injuries is one folder → two resources; prefabs is a UUID multimap).

pub(in crate::states::load) mod injuries;
mod params;
mod poll;
pub(in crate::states::load) mod prefab;

pub(in crate::states::load) use injuries::redrive_injuries_on_asset_event;
pub(in crate::states::load) use poll::poll_and_resolve;
pub(in crate::states::load) use prefab::redrive_prefabs_on_asset_event;

#[cfg(test)]
pub(crate) mod hot_reload_test_support;

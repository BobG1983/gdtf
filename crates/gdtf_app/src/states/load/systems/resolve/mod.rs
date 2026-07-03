//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.
//!
//! The orchestrator [`poll_and_resolve`] drives one poll per frame; each BESPOKE
//! per-asset FOLDER-resolve concern lives in its own sibling module: the theme
//! (inline in the orchestrator + its `fall_back` helper, in `poll`), the
//! attachments registry (GTW-549), the injuries registry + tables (GTW-437, one
//! folder → two resources), and the UUID-keyed prefab multimap (GTW-489). The
//! `params` module holds the two `SystemParam` bundles the orchestrator reads.
//!
//! GTW-564 moved the four SINGLE-FILE chains (situation + combat / stat /
//! procgen tuning) onto the generic hot-RON seam, and GTW-570 moved the seven
//! FOLDER content families (ranged/melee weapons, armor, fields, gangs, terrain
//! and theme defs) onto the generic content-family seam — their resolve/redrive
//! modules are gone; the Load plugin registers each with one ext call. The
//! three modules here are the DECLARED GTW-570 exclusions (attachments adoption
//! belongs to the GTW-579 rider; injuries and prefabs stay bespoke by design).

pub(in crate::states::load) mod attachments;
pub(in crate::states::load) mod injuries;
mod params;
mod poll;
pub(in crate::states::load) mod prefab;

pub(in crate::states::load) use attachments::redrive_attachments_on_asset_event;
pub(in crate::states::load) use injuries::redrive_injuries_on_asset_event;
pub(in crate::states::load) use poll::poll_and_resolve;
pub(in crate::states::load) use prefab::redrive_prefabs_on_asset_event;

#[cfg(test)]
pub(super) mod hot_reload_test_support;

//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.
//!
//! The orchestrator [`poll_and_resolve`] drives one poll per frame; each per-asset
//! resolve concern lives in its own sibling module: the theme (inline in the
//! orchestrator + its `fall_back` helper, in `poll`), the tuning, the situation, the
//! weapons registry, and the armor registry (GTW-269). The `params` module holds the
//! two `SystemParam` bundles the orchestrator reads.

mod armor;
mod params;
mod poll;
mod situation;
mod tuning;
mod weapons;

pub(in crate::states::load) use poll::poll_and_resolve;

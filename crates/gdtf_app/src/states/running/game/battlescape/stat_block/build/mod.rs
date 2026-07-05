//! The shared stat-block builder (GTW-278 / GTW-274): spawns ONE ganger stat block
//! (portrait, name, faction, stance, TU/HP bars, Wounds pips, wound-name list) and
//! returns its root [`Entity`](bevy::prelude::Entity) with the [`StatBlockRefs`](super::components::StatBlockRefs) handle stamped on it.
//!
//! Both the status panel and the hover panel call [`spawn_stat_block`] to build their
//! block, then parent it under their own root — DRY: the render of a ganger's vitals
//! lives here once. The widgets are spawned at empty / zero values; the panel's
//! per-update [`update_stat_block`](super::update_stat_block) repaints them by MUTATING
//! the stored widget entities ([[ui-mutate-not-respawn]]).

mod bars;
mod block;
mod lists;

pub(in crate::states::running::game::battlescape) use block::spawn_stat_block;

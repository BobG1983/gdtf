//! The shared ganger **stat block** (GTW-278 / GTW-274) — the ONE render of a ganger's
//! portrait / name / faction / stance / TU+HP bars / Wounds pips / wound-name list, built
//! once and reused by BOTH the top-left status panel (`SelectedShooter`) and the top-right
//! inspect panel (`InspectTarget`). DRY: the render lives here, the panels only own
//! their root, lifecycle, and target-resolution.
//!
//! - [`spawn_stat_block`] builds one block (the [`build`] submodule).
//! - [`update_stat_block`] mutates a block from a ganger's CURRENT components in place
//!   ([[ui-mutate-not-respawn]], the [`update`] submodule).
//! - [`portrait`] owns the deterministic [`GangerName`](gdtf_battle_sim::GangerName) →
//!   face mapping ([`PortraitIndex`]) + the `bevy_ui` portrait node builder.
//! - [`labels`] owns the pure name / faction / stance / wound-name format helpers.
//! - [`colors`] owns the mockup-approximate bar / pip colors.
//!
//! UI/view only — it reads sim components, owns no combat rule, and never writes the sim.

mod build;
mod colors;
mod components;
mod labels;
mod portrait;
mod update;

#[cfg(test)]
mod test;

pub(in crate::scenes::running::game::battlescape) use build::spawn_stat_block;
pub(in crate::scenes::running::game::battlescape) use components::StatBlockRefs;
pub(in crate::scenes::running::game::battlescape) use update::{
    StatBlockData, StatBlockWidgets, clear_stat_block, update_stat_block,
};

// Test-support-only re-export of the per-widget stat-block markers, widened to `pub`
// under `test-support` so the external integration tests can name them through
// `crate::test_support` to assert each widget's state (the status-panel per-line marker
// precedent). Gated so the production binary build stays `unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        StatFaction, StatHpBar, StatHpLabel, StatName, StatPortrait, StatStance, StatTuBar,
        StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips,
    };
}
// Test-support-only re-export of the deterministic portrait derivation, so the integration
// test can compute the expected face index from the same rule.
#[cfg(feature = "test-support")]
crate::support_use!(portrait::portrait_index_for_name;);

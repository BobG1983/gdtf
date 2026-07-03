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

pub(in crate::states::running::game::battlescape) use build::spawn_stat_block;
pub(in crate::states::running::game::battlescape) use components::StatBlockRefs;
pub(in crate::states::running::game::battlescape) use update::{
    StatBlockData, StatBlockWidgets, clear_stat_block, update_stat_block,
};

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the per-widget
/// stat-block markers (GTW-278) plus the deterministic portrait derivation (so the
/// integration test computes the expected face index from the same rule), named by the
/// external tests through `crate::test_support`. The crate-root ledger
/// (`src/test_support.rs`) re-exports these by explicit name directly from here — no
/// intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`) because the parent
/// chain is `pub(crate)`, so a `pub mod` here trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::{
        components::{
            StatFaction, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList, StatName,
            StatPortrait, StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList,
            StatWoundsPips,
        },
        portrait::portrait_index_for_name,
    };
}

//! The battlescape inspect panel (GTW-274): the TWIN of the status panel, an ABSOLUTE
//! fixed-% (`Val::Vw`/`Val::Vh`) overlay anchored TOP-RIGHT, hovering OVER the map (it
//! contributes NOTHING to the world-map viewport inset — only the bottom bar reduces the map).
//! It inspects the EFFECTIVE inspect target (pinned-else-hovered, GTW-300; the live cursor cell
//! while nothing is pinned) — reading the
//! [`InspectTarget`](gdtf_battle_input::InspectTarget) + the
//! [`OccupancyGrid`](gdtf_battle_sim::occupancy::OccupancyGrid):
//!
//! - a hovered GANGER → the shared [`stat_block`](super::stat_block), with its NAME line
//!   color tinted by the ganger's `Faction` (enemy red-ish, player the normal theme color);
//! - a hovered non-floor OBJECT (wall / cover) → an object stat block: a title, a labeled
//!   Integrity bar, and labeled Hardness / Protection / Height-band lines (GTW-295);
//! - bare floor / nothing → the panel is `Visibility::Hidden`.
//!
//! Pure VIEW; mutate-in-place on `InspectTarget` change ([[ui-mutate-not-respawn]]). It
//! mirrors the status-panel 4-file pattern (`mod` / `plugin` / `components` / `systems`)
//! and the same `BattleRunning` lifecycle.

mod components;
mod plugin;
mod shadow;
mod systems;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeInspectPanelScenePlugin;

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the inspect-panel's
/// root + host + object markers (GTW-274) the external integration tests (`status_panel`,
/// `real_battle_panel`) name through `crate::test_support`. Nothing in the binary reads
/// these re-exports (the panel's own systems reach the markers via the internal
/// `components::` path), so the module is `test-support`-gated. The crate-root ledger
/// (`src/test_support.rs`) re-exports these by explicit name directly from here — no
/// intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`) because the parent
/// chain is `pub(crate)`, so a `pub mod` here trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    };
}

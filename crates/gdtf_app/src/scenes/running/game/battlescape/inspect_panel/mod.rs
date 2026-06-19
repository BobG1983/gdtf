//! The battlescape inspect panel (GTW-274): the TWIN of the status panel, an ABSOLUTE
//! fixed-% (`Val::Vw`/`Val::Vh`) overlay anchored TOP-RIGHT, hovering OVER the map (it
//! contributes NOTHING to the world-map viewport inset — only the bottom bar reduces the map).
//! It inspects the EFFECTIVE inspect target (pinned-else-hovered, GTW-300; the live cursor cell
//! while nothing is pinned) — reading the
//! [`InspectTarget`](gdtf_battle_input::InspectTarget) + the
//! [`OccupancyGrid`](gdtf_battle_sim::OccupancyGrid):
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
mod systems;

#[cfg(test)]
mod test;

pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeInspectPanelScenePlugin;

// Test-support-only re-export of the inspect-panel's root + host + object markers, widened to
// `pub` under `test-support` so the external integration tests (`status_panel`,
// `real_battle_panel`) can name them through `crate::test_support`. Gated `test-support` so
// the production binary build stays `unused_imports`/`unreachable_pub`-clean: as of the
// GTW-275 overlay overhaul nothing in the binary reads `InspectPanelRoot` through this
// re-export (the panel's own spawn / despawn / update systems and the in-crate `test` module
// reach it via the internal `components::` path), so an UNCONDITIONAL re-export would be an
// unused import in the binary (caught by `dbuild`, masked by the `--workspace` clippy's
// feature unification).
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    };
}

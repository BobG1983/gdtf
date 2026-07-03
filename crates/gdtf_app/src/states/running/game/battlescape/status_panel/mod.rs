//! The battlescape status HUD panel (GTW-278): a themed `gdtf_ui` panel showing the
//! selected player ganger's shared stat block (portrait / name / faction / stance / TU+HP
//! `ProgressBar`s / Wounds `Pips` / wound-name list — the
//! [`stat_block`](super::stat_block)). The GTW-278 rework of the GTW-252 plain-text panel:
//! the `LifeState` + `Weapon` lines are GONE (user-directed), the name / faction / stance are
//! kept, and the bars / pips / portrait are the shared widgets. UI/view only: it reads the
//! sim's vital components + the input crate's `SelectedShooter`, owns no combat rule, and
//! changes no sim/input.

mod components;
mod plugin;
// `pub(crate)` so the crate-root test-support ledger can name the readout's own
// `test_support` submodule directly (GTW-569 one-hop ledger — the `StabilityBar` marker
// no longer climbs through here).
pub(crate) mod stability_readout;
mod systems;

// As of GTW-275 the status panel is an absolute fixed-% overlay that does NOT feed the
// world-map viewport inset (only the bottom bar reduces the map), so the `StatusPanelRoot`
// marker no longer needs re-exporting to the battlescape neighborhood: the panel's own spawn
// / despawn systems reach it via the internal `components::` path, and no sibling layout code
// reads it. Re-exporting it here would be an unused import (deny `unused_imports`).
pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeStatusPanelScenePlugin;

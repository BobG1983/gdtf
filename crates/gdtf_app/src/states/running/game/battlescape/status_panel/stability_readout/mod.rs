//! The status-panel **stability readout** (GTW-345): a "STAB" caption above a
//! [`ProgressBar`](gdtf_ui::spawn_progress_bar) showing the selected shooter's steadiness
//! (fuller bar = steadier shot), a SIBLING row under the status-panel root.
//!
//! - [`spawn_stability_readout`] builds the row at empty (the [`spawn`] submodule), the
//!   status-panel spawn parents it under its root.
//! - [`update_stability_readout`] repaints the bar fill from the selected shooter's
//!   [`stability_for`](gdtf_battle_sim::stability_for) [`ConeMult`](gdtf_battle_sim::ConeMult)
//!   in place (the [`update`] submodule, [[ui-mutate-not-respawn]]).
//! - [`components`] owns the bar marker + the named [`Steadiness`](components::Steadiness)
//!   presentation value.
//!
//! UI/view only — it reads the sim's authoritative `stability_for` composer + the on-entity
//! shooter / weapon components + the model cover / tuning resources, re-derives NO §1a math,
//! and never writes the sim.

mod components;
mod spawn;
mod update;

pub(in crate::states::running::game::battlescape::status_panel) use spawn::spawn_stability_readout;
pub(in crate::states::running::game::battlescape::status_panel) use update::update_stability_readout;

// Test-support-only re-export of the stability bar marker (GTW-345), widened to `pub` under
// `test-support` so the external integration test can name it through `crate::test_support`
// to assert the readout's fill (the stat-block per-widget marker precedent). Gated so the
// production binary build stays `unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use!(components::StabilityBar;);

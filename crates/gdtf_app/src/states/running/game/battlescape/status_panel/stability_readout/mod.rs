//! The status-panel **stability readout** (GTW-345): a "STAB" caption above a
//! [`ProgressBar`](gdtf_ui::spawn_progress_bar) showing the selected shooter's steadiness
//! (fuller bar = steadier shot), a SIBLING row under the status-panel root.
//!
//! - [`spawn_stability_readout`] builds the row at empty (the [`spawn`] submodule), the
//!   status-panel spawn parents it under its root.
//! - [`update_stability_readout`] repaints the bar fill from the selected shooter's
//!   [`stability_for`](gdtf_battle_sim::aim::stability_for) [`ConeMult`](gdtf_battle_sim::stability::ConeMult)
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

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the stability bar
/// marker (GTW-345) the external integration test names through `crate::test_support` to
/// assert the readout's fill. The crate-root ledger (`src/test_support.rs`) re-exports it
/// by explicit name directly from here — no intermediate `mod.rs` climb. `pub(crate)` on
/// the module (not `pub`) because the parent chain is `pub(crate)`, so a `pub mod` here
/// trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::StabilityBar;
}

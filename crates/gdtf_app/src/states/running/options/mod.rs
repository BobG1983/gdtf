//! The Options screen scene (GTW-637): the `bsn!` + `gdtf_ui`-widgets pilot.
//!
//! Wiring only (module-layout rule 2). The scene owns its settings model
//! ([`settings`]), its entity markers ([`components`]), its systems ([`systems`]),
//! and the scene plugin ([`plugin`]); see each submodule for detail.

mod components;
mod plugin;
mod settings;
mod systems;

pub(in crate::states::running) use plugin::OptionsScenePlugin;

/// Test-support re-exports for this scene (GTW-569 one-hop ledger): the screen's
/// entity markers the external integration tests name through
/// `crate::test_support`; internal code names them via the direct `components::`
/// path. The crate-root ledger (`src/test_support.rs`) re-exports these by explicit
/// name directly from here. `pub(crate)` on the module (not `pub`) because the
/// parent chain is `pub(crate)`, so a `pub mod` here trips `unreachable_pub`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{
        ContinueButton, OptionsScreenRoot, OptionsTitle, SoundToggle, SoundValueLabel,
    };
}

mod plugin;
mod systems;
pub(in crate::states::running) use plugin::MenuScenePlugin;

mod components;
/// Test-support re-exports for this scene (GTW-569 one-hop ledger): the menu button /
/// title markers (GTW-145) the external integration tests name through
/// `crate::test_support`; internal code names them via the direct `components::` path.
/// The crate-root ledger (`src/test_support.rs`) re-exports these by explicit name
/// directly from here — no intermediate `mod.rs` climb. `pub(crate)` on the module (not
/// `pub`) because the parent chain is `pub(crate)`, so a `pub mod` here trips the
/// workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{
        BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    };
}

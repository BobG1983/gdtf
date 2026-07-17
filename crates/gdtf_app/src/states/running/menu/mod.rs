mod plugin;
mod systems;
pub(in crate::states::running) use plugin::MenuScenePlugin;

// The start-battle navigation request (GTW-742): the Battlescape button and the
// dev-only `net_qa` network consumer both write `StartBattleRequested`, and
// `apply_start_battle` performs the `Menu → Game` transition. `pub(crate)` on the
// message so the network consumer (`crate::dev::net_qa`) can name it; the consumer
// system stays scene-local (only the menu plugin registers it).
mod start_battle;
pub(crate) use start_battle::StartBattleRequested;
pub(in crate::states::running::menu) use start_battle::apply_start_battle;

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

mod plugin;
mod systems;
pub(in crate::states::running) use plugin::MenuScenePlugin;

// The start-battle navigation request (GTW-742): the Battlescape button writes
// `StartBattleRequested` and `apply_start_battle` performs the `Menu → Game` transition.
// `pub(crate)` on the message so the menu plugin can name it from its sibling module; the
// consumer system stays scene-local (only that plugin registers it). GTW-943 removed the
// second writer — the `net_qa` start-battle consumer went with the request it served — and
// left the message's own `test-support` widening, which is how a suite drives the real
// descent (see `test_support` below).
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
    // GTW-943: the start-battle request itself. The `StartBattle` wire variant that used
    // to carry a suite into a battle went with the rest of the pre-command vocabulary, so
    // a test drives the SAME message the Battlescape button writes instead — the real
    // navigation path, one step earlier.
    pub use super::start_battle::StartBattleRequested;
}

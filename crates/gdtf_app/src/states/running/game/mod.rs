mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameScenePlugin;
mod resources;

// The game sub-state enum lives in the folder it governs (GTW-321); this
// `support_use!` carries it up toward `crate::states::GameState`.
mod game_state;
crate::support_use!(game_state::GameState;);

mod setup;
pub(in crate::states::running::game) use setup::GameSetupScenePlugin;

mod hivescape;
pub(in crate::states::running::game) use hivescape::GameHiveScapeScenePlugin;

// `pub(crate)` so the crate-root test-support ledger can name the panel `test_support`
// submodules under it (GTW-569 one-hop ledger — markers no longer climb through here).
pub(crate) mod battlescape;
pub(in crate::states::running::game) use battlescape::GameBattleScapeScenePlugin;
// The battlescape + aftermath sub-state enums climb from `battlescape` toward
// `crate::states::{BattleScapeState, AfterMathState}` (GTW-321 co-location chain).
// Panel/test markers do NOT climb through here — the crate-root test-support ledger names
// each panel's own `test_support` submodule directly (GTW-569 one-hop ledger).
crate::support_use!(battlescape::BattleScapeState;);
crate::support_use!(battlescape::AfterMathState;);

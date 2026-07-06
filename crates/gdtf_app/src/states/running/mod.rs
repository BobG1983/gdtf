mod plugin;
mod systems;
pub(in crate::states) use plugin::RunningScenePlugin;
mod resources;

// The running sub-state enum lives in the folder it governs (GTW-321); this
// `support_use!` carries it up toward `crate::states::RunningState`.
mod running_state;
crate::support_use!(running_state::RunningState;);

// `pub(crate)` so the crate-root test-support ledger can name the scene's own
// `test_support` submodule directly (GTW-569 one-hop ledger — the menu markers no longer
// climb through here).
pub(crate) mod menu;
pub(in crate::states::running) use menu::MenuScenePlugin;

// `pub(crate)` so the crate-root test-support ledger can name the panel `test_support`
// submodules under it (GTW-569 one-hop ledger — markers no longer climb through here).
pub(crate) mod game;
pub(in crate::states::running) use game::GameScenePlugin;
// The game + battlescape + aftermath sub-state enums climb from `game` toward
// `crate::states::{GameState, BattleScapeState, AfterMathState}` (GTW-321 chain).
// Panel/test markers do NOT climb through here — the crate-root test-support ledger names
// each panel's own `test_support` submodule directly (GTW-569 one-hop ledger).
crate::support_use!(game::GameState;);
crate::support_use!(game::BattleScapeState;);
crate::support_use!(game::AfterMathState;);

// `pub(crate)` so the crate-root test-support ledger can name the scene's own
// `test_support` submodule directly (GTW-569 one-hop ledger — the gang-editor model +
// markers no longer climb through here).
pub(crate) mod gang_editor;
pub(in crate::states::running) use gang_editor::GangEditorScenePlugin;

// The DEV-ONLY procgen STEP/AUTO visualizer (GTW-434). The WHOLE module — model, components,
// systems, and scene plugin — is `#[cfg(debug_assertions)]`-gated so it compiles out of a
// release build entirely (C4); a release build never sees it. `pub(crate)` so the
// crate-root test-support ledger can name the scene's own `test_support` submodule
// directly (GTW-569 one-hop ledger — the visualizer markers no longer climb through here).
#[cfg(debug_assertions)]
pub(crate) mod procgen_viz;
#[cfg(debug_assertions)]
pub(in crate::states::running) use procgen_viz::ProcgenVizScenePlugin;

mod options;
pub(in crate::states::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::states::running) use quit::QuitScenePlugin;

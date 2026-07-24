mod plugin;
mod systems;
pub(in crate::states) use plugin::RunningScenePlugin;
// GTW-764: lift the UI-camera marker to `crate::states::running::UiCamera` so the DEV-ONLY
// `net_qa` offscreen-capture retarget system can name it crate-wide. GTW-819 adds a second
// consumer under `dev_tools` (`crate::dev::ui_coexistence::egui_context`, which binds the
// primary egui context to this camera), so the gate is the UNION of the two consumers' gates:
// with neither feature the re-export is unused (and `-D unused-imports` red).
#[cfg(any(all(debug_assertions, feature = "net_qa"), feature = "dev_tools"))]
pub(crate) use systems::UiCamera;
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
// `test_support` submodule directly (GTW-569 one-hop ledger — the Options screen
// markers no longer climb through here), mirroring `menu` / `game`.
pub(crate) mod options;
pub(in crate::states::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::states::running) use quit::QuitScenePlugin;

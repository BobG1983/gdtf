mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameBattleScapeGenerationScenePlugin;
mod resources;

mod battle_sim;

// The GTW-419 LOADING SCREEN: the themed full-viewport overlay shown while the sim assembles the
// level + builds the battle (the Generation phase), guaranteeing no partial-level frame reaches
// the player. View-only — no sim change.
mod loading_screen;
// Test-support-only re-export of the loading-screen root marker (GTW-419), gated so the binary
// build stays `unused`/`unreachable_pub`-clean (the battlescape marker re-export chain
// precedent). `support_use!` widens it to `pub` under `test-support`. The AC tests name it
// through `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use!(loading_screen::LoadingScreenRoot;);

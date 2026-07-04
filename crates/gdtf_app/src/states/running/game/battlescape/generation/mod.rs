mod plugin;
pub(in crate::states::running) use plugin::GameBattleScapeGenerationScenePlugin;
mod resources;

mod battle_sim;

// The GTW-419 LOADING SCREEN: the themed full-viewport overlay shown while the sim assembles the
// level + builds the battle (the Generation phase), guaranteeing no partial-level frame reaches
// the player. View-only — no sim change. `pub(crate)` so the crate-root test-support ledger can
// name `loading_screen::test_support` directly (GTW-569 one-hop ledger — no climb through here).
pub(crate) mod loading_screen;

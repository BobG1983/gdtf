mod plugin;
pub(in crate::states::running) use plugin::GameBattleScapeGenerationScenePlugin;
mod resources;

// `pub(crate)`, not private: the GTW-655 dev-tools stepper (`crate::dev::procgen_stepper`)
// names `battle_sim::{outcome_from_emitted, ProcgenOutcome}` to finish its staged drive
// through the SAME merge + finding-conversion logic `request_battle_setup` uses.
pub(crate) mod battle_sim;

// The GTW-419 LOADING SCREEN: the themed full-viewport overlay shown while the sim assembles the
// level + builds the battle (the Generation phase), guaranteeing no partial-level frame reaches
// the player. View-only — no sim change. `pub(crate)` so the crate-root test-support ledger can
// name `loading_screen::test_support` directly (GTW-569 one-hop ledger — no climb through here).
pub(crate) mod loading_screen;

//! GTW-236 (lifecycle): the battlescape PERSISTS in `BattleRunning` + the drive proof.
//!
//! `BattleRunning` no longer auto-walks back out: the placeholder 3-tick turn-budget
//! gate (`BattleRunTurnBudget` + its decrement/track systems) is GONE. The battle now
//! RESTS in `BattleScapeState::BattleRunning` indefinitely and leaves ONLY once the
//! explicit end-signal marker `BattleRunningComplete` is inserted — only then does the
//! existing `move_on` advance to `AnimateOut`. The victory census / flee button (sibling
//! slices) are what insert it in the running game; these tests insert it through the
//! `test_support` surface to stand in for that. They are headless `MinimalPlugins`,
//! driven to the live battle by the shared [`gdtf_test_utils::BattleAppBuilder`] (which
//! seeds the persistent `Load` resources a `MinimalPlugins` app has no `AssetServer` to
//! load, descends past the menu, and rests at `BattleRunning`). They are
//! *pin-discriminating*: each assertion re-encodes one acceptance criterion so a
//! regression turns the test red — a re-added auto-exit turns the persistence assertions
//! red.
//!
//! GTW-324: the situation + fixture gangers come from the canonical
//! [`gdtf_battle_sim::test_support`] builders ([`GangerSpawnBuilder`] /
//! [`SituationBuilder`]); the drive-to-battle sequence is the crate-central
//! [`BattleAppBuilder`]. The file no longer hand-rolls its weapon/armor registries, its
//! `ganger_at`, or its `walk_app` / `drive_to_battle_running` walk.
//!
//! The drive proof (AC1) sets the battle up via the REAL message-driven path: the builder
//! inserts the authored situation BEFORE driving, the app's `OnEnter(Generation)` sends
//! `SetupBattleRequested`, and the sim's `setup_battle` spawns the gangers via `Commands`.
//! The test then QUERIES the spawned ganger entities (by faction) off the world — it never
//! takes a `&mut World` in a helper and never hand-spawns an entity that bypasses
//! `setup_battle`. The accepted `gdtf_app` harness idiom — `app.world_mut()` /
//! `app.world()` in the TEST BODY for `insert_resource` / `write_message` / queries — is
//! used throughout (every sibling `gdtf_app` integration test does the same).

mod census_outcomes;
mod faction_outcomes;
mod fire_drive;
mod harness;
mod persistence;

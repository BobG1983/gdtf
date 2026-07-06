//! GTW-207 (E10.5): `BattleSimPlugin` drives the render-free authoritative sim
//! into the running app — on entry to `BattleScapeState::Generation` it seeds the
//! battle RNG streams (GTW-14), builds the battle from the authored `Situation` via the
//! authoritative `setup_battle`, gates Generation's state advance on REAL setup
//! success, and cleans the battle-lifetime resources only when the battle ends.
//!
//! All tests are headless `MinimalPlugins` (via [`GdtfTestAppBuilder`]) — they
//! BYPASS the `Load` scene, so each injects the persistent `Load` resources it
//! relies on (a `GdtfTheme` + `CombatTuning` to pass the Load gate, and where the
//! test exercises a specific battlefield, a `LoadedSituation` fixture). They are
//! *pin-discriminating*: each assertion re-encodes one acceptance criterion so a
//! regression turns the test red.

mod generation_gate;
mod harness;
mod seed_logging;
mod setup;

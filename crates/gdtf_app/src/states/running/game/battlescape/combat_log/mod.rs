//! The battlescape combat-text LOG (GTW-328, slice 3, bottom-left, ABOVE the weapon panel): a
//! battle-scoped strip of the most-recent combat events that scroll up and fade.
//!
//! GTW-572 (C5/C6): the log is a FORWARDER → APPENDER message seam. The forwarder half
//! lives in the PRESENTER since GTW-620 (`gdtf_battle_presenter`'s
//! `actors/fx/fct/log_event/`, beside the vocabulary + classifier it feeds): one thin
//! forwarder per log SOURCE drains its sim fact message, resolves each
//! [`Entity`](bevy::prelude::Entity) to a ganger name at that boundary, and writes a
//! buffered [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent). This module is the
//! `bevy_ui` half: the ONE appender (ordered `.after` the presenter's exported
//! [`CombatLogSystems::Forward`](gdtf_battle_presenter::CombatLogSystems) set) drains
//! those events, classifies them through the shared
//! [`classify_log_event`](gdtf_battle_presenter::classify_log_event), and renders each
//! resulting line as a UI text node that fades over a tuned lifetime and FIFO-despawns when
//! the visible count overflows the tuned cap. Coverage is ALL state changes (the Q2
//! ruling): fire declarations, movement (and suppressed rejections), staggered shot
//! outcomes, reloads, turn boundaries, injuries, falls, melee damage, terminal deaths,
//! suppression, armor breaks — and the DOT / field / bleed afflictions ONCE at affliction
//! start (their per-tick drain signals never log).
//!
//! UI/view only: it reads the sim's combat-event messages + ganger names, owns no combat rule,
//! and writes nothing back. Mutate-not-respawn — the per-frame fade UPDATES each existing line's
//! alpha; a new line is spawned only on a NEW event, and a line is despawned only on TTL /
//! overflow ([[ui-mutate-not-respawn]]). Its feel (max lines / TTL / fade / panel width) is the
//! hot-reloadable [`CombatLogTuning`] RON table.

mod components;
mod plugin;
mod systems;
mod tuning;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeCombatLogScenePlugin;

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the log ROOT +
/// per-LINE markers (GTW-328) the AC test names through `crate::test_support` to assert
/// the log gains the expected line children + FIFO overflow. The crate-root ledger
/// (`src/test_support.rs`) re-exports these by explicit name directly from here — no
/// intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`) because the parent
/// chain is `pub(crate)`, so a `pub mod` here trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{CombatLogLine, CombatLogRoot};
}

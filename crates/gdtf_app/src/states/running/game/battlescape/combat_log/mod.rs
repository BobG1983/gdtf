//! The battlescape combat-text LOG (GTW-328, slice 3, bottom-left, ABOVE the weapon panel): a
//! battle-scoped strip of the few most-recent combat events that scroll up and fade.
//!
//! The log drains the five sim combat-event messages (movement, shot declarations, shot
//! outcomes, reloads, turn boundaries), resolves each [`Entity`](bevy::prelude::Entity) to a
//! ganger name, classifies them through the shared
//! [`classify_log_event`](gdtf_battle_presenter::classify_log_event) (slice 2), and renders each
//! resulting line as a UI text node that fades over a tuned lifetime and FIFO-despawns when the
//! visible count overflows the tuned cap.
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

// Test-support-only re-export of the log ROOT + per-LINE markers (GTW-328), gated so the binary
// build stays `unused`/`unreachable_pub`-clean (the weapon-panel marker re-export chain
// precedent). The AC test names these through `crate::test_support` to assert the log gains the
// expected line children + FIFO overflow.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{CombatLogLine, CombatLogRoot};
}

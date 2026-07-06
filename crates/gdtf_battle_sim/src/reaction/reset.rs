//! The turn-boundary per-turn cap reset — zero every watcher's [`ReactionsUsed`]
//! counter when a turn starts (GTW-468 C6).

use bevy::prelude::{MessageReader, Query};

use crate::tuning::ReactionsUsed;

/// **Reset** every watcher's per-turn interrupt counter at a turn boundary — the §8 cap
/// "max interrupts this enemy turn" applies AFRESH each turn (GTW-468 C6).
///
/// Drains [`MessageReader<TurnStarted>`] (the turn-cycle boundary
/// [`dispatch_end_turn`](crate::turn::dispatch_end_turn) emits per advance) and, when ANY
/// turn started this tick, zeroes every ganger's [`ReactionsUsed`]
/// ([`reset`](crate::tuning::ReactionsUsed::reset)).
///
/// **Why reset every boundary (not only the enemy's).** §8 frames the cap as "this enemy
/// turn", but reaction fire is faction-symmetric (AC5 — a player watcher reacts on the enemy
/// turn AND an enemy watcher reacts on the player turn). The cap is per-WATCHER and a
/// watcher only reacts during its OPPONENT's turn, so resetting EVERY watcher at EVERY turn
/// boundary generalizes "this enemy turn" to "this turn relative to the watcher" correctly:
/// a watcher's counter is zeroed before each turn in which it could react, and never
/// double-counts across two of its own opponent's turns. This is the defensible default
/// (DESIGN FORK, flagged); a per-faction reset would be a no-op refinement.
///
/// Ordered `.after(`[`dispatch_end_turn`](crate::turn::dispatch_end_turn)`)` (so the
/// boundary's [`TurnStarted`](crate::turn::TurnStarted) is buffered) in
/// [`SimActsPlugin`](crate::acts::SimActsPlugin). Its own independent reader, so it never
/// steals the boundary from the combat-log / bleed readers (the `tick_bleed`
/// `enemy_phase_started` precedent). Param-only — `Query` / `MessageReader`, no `&mut World`
/// (`bevy-traps.md` #7).
pub fn reset_reactions_used(
    mut turns: MessageReader<crate::turn::TurnStarted>,
    mut used: Query<&mut ReactionsUsed>,
) {
    // A turn boundary crossed this tick iff any TurnStarted was emitted. The reset is
    // idempotent (zeroing is the same for N boundaries as for one), so drain-then-act-once.
    let mut crossed = false;
    for _turn in turns.read() {
        crossed = true;
    }
    if !crossed {
        return;
    }
    for mut counter in &mut used {
        counter.reset();
    }
}

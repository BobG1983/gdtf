//! The runtime wiring of the §9 bleed-out clock into the turn cycle (GTW-336).
//!
//! [`tick_bleed`](crate::effects::bleed::tick_bleed) is the pure per-round drain (the E3.7
//! slice, GTW-189), but until this slice it ran ONLY in unit tests — no production
//! schedule registered it, so a live battle's Downed gangers never bled, never died
//! to the clock, and the presenter's `"Bleeding"` floating-combat-text pop never fired
//! (the consequence reader is gated on a `Messages<Bleeding>` buffer nothing in the
//! runtime created). This module closes that gap: it supplies the run condition that
//! fires the drain **once per full round, at the enemy-phase start**
//! (`docs/combat/resolution.md` §9: "once per full round (`tick_bleed`, ticked at the
//! enemy-phase start)"), so the [`SimActsPlugin`](crate::acts::SimActsPlugin) can wire
//! `tick_bleed` into the live [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems)
//! band.
//!
//! ## Why the enemy-phase start is "once per full round"
//!
//! The turn-cycle engine ([`dispatch_end_turn`](crate::turn::dispatch_end_turn),
//! GTW-309) processes one [`EndTurnRequested`](crate::acts::EndTurnRequested) by handing
//! the turn off the ending team to the OTHER team, emitting one
//! [`TurnStarted`](crate::turn::TurnStarted) for the now-active team. A full round is one
//! player → enemy → player cycle, now spanning TWO end-turn signals (the player's, then the
//! GTW-70 enemy-AI brain's), so the **enemy** `TurnStarted` (the `now_active !=
//! PlayerFaction` boundary) is emitted exactly once per full round — when the player ends
//! its turn and control hands off to the enemy. [`enemy_phase_started`] keys the bleed-out
//! drain off that boundary.

use bevy::prelude::{MessageReader, Res};

use crate::{battle::PlayerFaction, turn::TurnStarted};

/// A run condition: did the **enemy phase** start this frame? (GTW-336.)
///
/// Returns `true` iff a [`TurnStarted`](crate::turn::TurnStarted) the turn-cycle engine
/// emitted this frame announces a turn for a team that is NOT the
/// [`PlayerFaction`](crate::battle::PlayerFaction) — the enemy-phase boundary
/// `docs/combat/resolution.md` §9 names as the once-per-full-round bleed tick. The
/// turn-cycle engine ([`dispatch_end_turn`](crate::turn::dispatch_end_turn)) advances the
/// active faction off the ending team to the other team, emitting one `TurnStarted` per
/// advance; the enemy one fires exactly once per full round (when the player ends its turn
/// and control hands off to the enemy), so gating
/// [`tick_bleed`](crate::effects::bleed::tick_bleed) on this condition drains the bleed-out clock
/// once per full round.
///
/// It owns its OWN independent [`MessageReader`](bevy::prelude::MessageReader) cursor, so
/// reading the buffer here does NOT consume the `TurnStarted` messages from the other
/// readers that key off the same boundary (the presenter's combat-log reader, GTW-328) —
/// each [`MessageReader`](bevy::prelude::MessageReader) tracks its own read position over
/// the shared double-buffer.
///
/// [`PlayerFaction`](crate::battle::PlayerFaction) is read as `Option<Res<PlayerFaction>>`
/// because it is battle-lifetime
/// (co-inserted with [`BattleInProgress`](crate::battle::BattleInProgress) /
/// [`ActiveFaction`](crate::turn::ActiveFaction)); without it there is no live battle, so
/// no enemy phase can have started — the condition is `false` and the gated
/// [`tick_bleed`](crate::effects::bleed::tick_bleed) stays inert (`bevy-traps.md` #1). It must run
/// `.after(`[`dispatch_end_turn`](crate::turn::dispatch_end_turn)`)` so the frame's
/// `TurnStarted` is already buffered when this reads it.
#[must_use]
pub fn enemy_phase_started(
    mut turns: MessageReader<TurnStarted>,
    player: Option<Res<PlayerFaction>>,
) -> bool {
    // No PlayerFaction => no live battle => no enemy phase could have started.
    let Some(player) = player else {
        return false;
    };
    let player = **player;
    // True iff ANY TurnStarted this frame announced a non-player (enemy) turn — the
    // enemy-phase boundary the once-per-round bleed tick keys off. `any` short-circuits
    // and advances this reader's own cursor (independent of every other reader).
    turns.read().any(|started| started.now_active != player)
}

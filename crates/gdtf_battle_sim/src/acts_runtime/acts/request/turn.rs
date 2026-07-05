//! The fieldless end-turn signal — [`EndTurnRequested`].

use bevy::prelude::Message;

/// An **end-turn** act was requested — the active team passes control to the other team
/// (GTW-309).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying **NO
/// payload**. End-turn is a GLOBAL turn signal, not a per-ganger act: which team's turn is
/// ending is tracked by the [`ActiveFaction`](crate::turn::ActiveFaction) resource, NOT a
/// message field — so this is deliberately FIELDLESS (no `{ actor: Entity }`), unlike the
/// per-ganger [`ReloadRequested`](super::reload::ReloadRequested) / [`MoveRequested`](super::movement::MoveRequested). A fieldless unit struct carries no
/// domain value, so the no-bare-types rule — which wraps *values* — does not apply; the
/// type's identity IS the signal. [`dispatch_end_turn`](crate::turn::dispatch_end_turn)
/// drains this and advances the turn cycle: it hands the turn to the other team (running
/// that team's turn-start TU regen) and STOPS there (GTW-70 removed the auto-pass). The
/// enemy turn is then driven by the GTW-70 enemy-AI brain
/// ([`enemy_ai_turn`](crate::ai::enemy_ai_turn)), which emits its OWN `EndTurnRequested` to
/// hand control back to the player.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndTurnRequested;

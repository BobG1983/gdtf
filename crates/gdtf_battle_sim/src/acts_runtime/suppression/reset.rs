//! The **suppression clear cadence** [`reset_suppression`] (GTW-526 C6, child of GTW-41).
//!
//! Suppression is faction-scoped and clears at the suppressed unit's OWN turn-start — so
//! a unit stays pinned through its opponent's whole turn (during which it cannot
//! reaction-fire) and shakes it off only when it next gets to act. This is the mirror of
//! the turn-start TU regen [`regen_team_tu`](crate::turn::regen_team_tu): on a
//! [`TurnStarted { now_active }`](crate::turn::TurnStarted), every ganger of the
//! now-active faction loses its [`Suppressed`] component.
//!
//! This is DELIBERATELY faction-scoped, NOT the faction-agnostic
//! [`reset_reactions_used`](crate::reaction::reset_reactions_used) cadence (which zeroes
//! EVERY watcher on EVERY boundary). Suppression must persist across the opponent's turn:
//! clearing everyone on every boundary would lift the pin the instant the opponent's turn
//! began, defeating the effect.

use bevy::prelude::{Commands, Entity, MessageReader, Query};

use crate::{
    ganger::{Faction, Suppressed},
    turn::TurnStarted,
};

/// **Clear** suppression from the faction whose turn just started (GTW-526 C6).
///
/// Drains [`MessageReader<TurnStarted>`](crate::turn::TurnStarted) (the turn-cycle
/// boundary [`dispatch_end_turn`](crate::turn::dispatch_end_turn) emits per advance) and,
/// for each now-active faction, removes [`Suppressed`] from every ganger of THAT faction
/// via [`Commands`](bevy::prelude::Commands). A ganger of another faction keeps its
/// suppression (it is still in its opponent's turn).
///
/// Faction-scoped like [`regen_team_tu`](crate::turn::regen_team_tu) (`if *faction ==
/// team`), NOT the faction-agnostic
/// [`reset_reactions_used`](crate::reaction::reset_reactions_used). Result: a unit stays
/// suppressed through the whole enemy turn and clears at its own faction's turn-start.
///
/// Its own independent [`MessageReader`](bevy::prelude::MessageReader) cursor (so it never
/// steals the boundary from the combat-log / bleed / cap-reset readers — the `tick_bleed`
/// / `reset_reactions_used` precedent). Ordered `.after(dispatch_end_turn)` (so the
/// boundary's [`TurnStarted`] is buffered) in
/// [`SimActsPlugin`](crate::acts::SimActsPlugin). Param-only (`MessageReader` / `Query` /
/// `Commands`) — no `&mut World` (`bevy-traps.md` #7).
pub fn reset_suppression(
    mut turns: MessageReader<TurnStarted>,
    suppressed: Query<(Entity, &Faction), bevy::prelude::With<Suppressed>>,
    mut commands: Commands,
) {
    for turn in turns.read() {
        let now_active = turn.now_active;
        for (entity, faction) in &suppressed {
            if *faction == now_active {
                commands.entity(entity).remove::<Suppressed>();
            }
        }
    }
}

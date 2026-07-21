//! The battle-start auto-select (GTW-255): [`auto_select_first_player_ganger`] sets the INITIAL
//! [`SelectedShooter`] once to the deterministic player-faction ganger — plus the GTW-729
//! stale-selection clear ([`clear_downed_selection`]) that drops a selection whose ganger just
//! went Downed / Dead, so auto-select then advances it to the next Alive one.

use bevy::prelude::*;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::selection::{
    order::cell_order_key,
    resources::{SelectedShooter, set_selection},
};

/// Sets the INITIAL [`SelectedShooter`] to the deterministic player-faction ganger when the
/// battle becomes live with nothing selected yet (GTW-255).
///
/// Fixes the "battle opens with no unit selected" play-test bug: at setup [`SelectedShooter`]
/// is `init_resource`-d to [`None`]. This system fills that EMPTY selection ONCE with one of
/// the player's own gangers, so the highlight + fire-mode default are live the moment the battle
/// opens — exactly as for a click selection.
///
/// Behaviour:
///
/// - **No-op unless the selection is empty.** It only acts when `**selected == None`; it NEVER
///   overrides a selection the player (or any other system) already made.
/// - **Deterministic ganger.** Among the ALIVE gangers whose [`Faction`] `==` [`PlayerFaction`],
///   it selects the one with the lowest [`Position`] cell by the `(level, y, x)` total ordering
///   ([`cell_order_key`]) — reproducible across runs, NOT the allocation-order [`Entity`] id.
/// - **Never an enemy, never a downed ganger.** It reads `Res<`[`PlayerFaction`]`>` and only
///   considers gangers whose own [`Faction`] equals it AND are [`Alive`](LifeState::Alive)
///   (GTW-729 — a Downed / Dead ganger is never a selectable ACTOR). No Alive player-faction
///   ganger present → the selection stays [`None`].
///
/// Together with [`clear_downed_selection`] this ADVANCES a stale selection: when the selected
/// ganger goes Downed, that system clears the selection to [`None`], and this system then refills
/// it with the next Alive player ganger (GTW-729 — "clear or advance to an Alive unit").
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<(`[`Entity`]`, &`[`Faction`]`,
/// &`[`Position`]`, Option<&`[`LifeState`]`>)>` + `Res<`[`PlayerFaction`]`>` + the
/// [`ResMut<SelectedShooter>`] write — no `&mut World`. Gated
/// `run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>))`
/// (`bevy-traps.md` #1), placed in [`InputSystems::Gather`](crate::InputSystems) ordered
/// `.before(`[`left_click_act`](crate::left_click_act)`)` and `.after(clear_downed_selection)`.
pub fn auto_select_first_player_ganger(
    gangers: Query<(Entity, &Faction, &Position, Option<&LifeState>)>,
    player: Res<PlayerFaction>,
    mut selected: ResMut<SelectedShooter>,
) {
    // Only ever FILL an empty selection — never override an existing one.
    if selected.is_some() {
        return;
    }
    let player_faction = **player;
    // The deterministic player-faction ALIVE ganger: the lowest `Position` cell by the
    // `(level, y, x)` total ordering. `min_by_key` returns `None` when the player has no Alive
    // gangers, leaving the selection empty (never an enemy, never a downed body). The life gate
    // is FAIL-OPEN via `Option<&LifeState>` — a ganger with no `LifeState` (a focused test
    // fixture; a real ganger always carries one) counts as selectable.
    let pick = gangers
        .iter()
        .filter(|(_, faction, _, life)| {
            **faction == player_faction && life.is_none_or(|life| *life.is_active())
        })
        .min_by_key(|(_, _, position, _)| cell_order_key(position))
        .map(|(entity, ..)| entity);
    if let Some(entity) = pick {
        set_selection(&mut selected, SelectedShooter::new(entity));
    }
}

/// Clears the [`SelectedShooter`] when the currently-selected ganger is no longer
/// [`Alive`](LifeState::Alive) — the GTW-729 stale-selection guard.
///
/// A ganger can be selected and THEN go Downed (or Dead) mid-turn — struck down by an enemy
/// reaction while it was the acting unit. A Downed ganger cannot act, so a selection stranded on
/// it is a dead selection: the action bar would offer acts the sim refuses. This system drops
/// such a selection the moment the selected ganger's [`LifeState`] leaves Alive (or the ganger
/// despawns — a Dead body is cleared from the world), so [`auto_select_first_player_ganger`],
/// ordered `.after` this one, refills the now-empty selection with the next Alive player ganger
/// the SAME update (clear → advance).
///
/// A no-op unless there is a selection AND that ganger is PRESENT with a non-Alive
/// [`LifeState`]: an empty selection, an Alive selection, or a selection whose ganger carries no
/// [`LifeState`] at all (a query miss — the fail-open case, since a real ganger always carries
/// one) is left untouched. The primary path is Downed: a ganger transitions Alive → Downed while
/// still present (a Dead ganger also passes through a present [`LifeState::Dead`] before
/// [`sync_dead_gangers`](gdtf_battle_sim::occupancy_sync::sync_dead_gangers) despawns it, so this
/// clears it that frame — the query miss after despawn needs no separate handling).
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<&`[`LifeState`]`>` + the
/// [`ResMut<SelectedShooter>`] write — no `&mut World`. Gated
/// `run_if(resource_exists::<BattleInProgress>)`, placed in
/// [`InputSystems::Gather`](crate::InputSystems) ordered
/// `.before(`[`auto_select_first_player_ganger`]`)` so the clear + refill land in one update.
pub fn clear_downed_selection(lifes: Query<&LifeState>, mut selected: ResMut<SelectedShooter>) {
    let Some(actor) = **selected else {
        return; // Nothing selected — nothing to strand.
    };
    // Clear ONLY when the selected ganger is PRESENT with a non-Alive LifeState (Downed / Dead).
    // A query miss (no LifeState, or already despawned) is FAIL-OPEN — leave the selection be.
    if let Ok(life) = lifes.get(actor)
        && !*life.is_active()
    {
        *selected = SelectedShooter::cleared();
    }
}

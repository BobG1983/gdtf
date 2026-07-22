//! The ONE classic-intent drain: [`dispatch_act_intents`] + the private `cycle_selection`
//! helper.

use bevy::prelude::*;
use gdtf_battle_presenter::{ActiveLevel, PlaybackGate, ViewMode};
use gdtf_battle_sim::{
    acts::{
        AimRequest, EndTurnRequested, ReloadRequested, SetAimingRequested, SetFacingRequested,
        SetStanceRequested,
    },
    ganger::{Aiming, Facing},
    prelude::{Stance, StanceKind},
};

use super::{ActIntent, ActWriters, PendingActIntent, SelectionCycleReads};
use crate::{
    SelectedShooter, cycle,
    intent::level::{LevelStep, step_level},
    selection::{CycleDirection, cycle_player_selection},
};

/// **Dispatch** the queued [`ActIntent`]s — the CLASSIC-intent drain over the shared
/// [`PendingActIntent`] queue (the Q5 invariant: per-act generic drains in one
/// explicitly-ordered `SystemSet`, same-frame semantics preserved — this drain is the
/// classic half; the contextual acts' per-act generic drains live in
/// [`crate::contextual`] and run `.before` this one).
///
/// Drains the [`PendingActIntent`] queue every update (gated on `BattleInProgress` by
/// the plugin) and interprets each intent. The no-act intents are handled directly:
///
/// - [`ActIntent::SelectionClear`] clears [`SelectedShooter`] to `None`.
/// - [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] step [`ActiveLevel`], clamped
///   to `0..MAX_LEVELS` ([`step_level`]): up saturates at `MAX_LEVELS - 1`, down
///   floors at `0`.
/// - [`ActIntent::ToggleFullView`] flips the presenter-owned
///   [`ViewMode`](gdtf_battle_presenter::ViewMode) between `DownToActive` and `FullView`
///   (GTW-521) — it does NOT touch [`ActiveLevel`]; the presenter's terrain draw + ganger
///   visibility re-run on the [`ViewMode`](gdtf_battle_presenter::ViewMode) change.
///
/// The act-bearing intents (GTW-227) emit the matching
/// `gdtf_battle_sim::acts::*Requested` for the [`SelectedShooter`], reading the
/// actor's CURRENT [`Stance`] / [`Facing`] / [`Aiming`] off the `actors` query and
/// stepping the authored [`crate::cycle`] order:
///
/// - [`ActIntent::StanceCycle`] emits [`SetStanceRequested`] for the
///   next-of-cycle [`StanceKind`] ([`cycle::next_stance`]).
/// - [`ActIntent::SetStance`] emits [`SetStanceRequested`] for the CARRIED
///   [`StanceKind`] directly (GTW-267 — the action-bar 3-toggle stance set).
/// - [`ActIntent::AimToggle`] emits [`SetAimingRequested`] toggling the actor's
///   current [`Aiming`] flag.
/// - [`ActIntent::FacingCycle`] emits [`SetFacingRequested`] for the next-of-cycle
///   [`Direction`](gdtf_battle_sim::ganger::Direction) ([`cycle::next_facing`]).
/// - [`ActIntent::Fire`] emits the carried [`FireRequested`](gdtf_battle_sim::acts::FireRequested) verbatim — the `can_fire`
///   guard already ran at the WRITE site.
/// - [`ActIntent::Move`] emits the carried [`MoveRequested`](gdtf_battle_sim::acts::MoveRequested) verbatim onto the move
///   writer (GTW-238) — the destination's terrain TU cost is the sim's move dispatch.
/// - [`ActIntent::Turn`] emits the carried [`SetFacingRequested`] verbatim onto the SAME
///   facing writer the [`FacingCycle`](ActIntent::FacingCycle) arm uses (GTW-238) — the
///   per-45deg-step turn TU cost is the sim's facing dispatch.
/// - [`ActIntent::Reload`] emits [`ReloadRequested`] for the [`SelectedShooter`]
///   (GTW-275) — a no-op with no selection; the per-weapon `reload_tu` cost is the sim's
///   reload dispatch.
/// - [`ActIntent::EndTurn`] emits the fieldless [`EndTurnRequested`] unconditionally
///   (GTW-309) — a GLOBAL turn signal needing no selection; the sim's `dispatch_end_turn`
///   advances the turn cycle and runs the next team's turn-start TU regen.
///
/// The GTW-458 SELECTION-CYCLE intents step the [`SelectedShooter`] through the player gang
/// in the shared `(z, y, x)` order ([`cell_order_key`](crate::selection::cell_order_key) — the exact order the auto-select
/// picks the first of), WRAPPING, ignoring enemy gangers, and MAKING a first selection from
/// `None`:
///
/// - [`ActIntent::SelectNext`] sets the WRAPPING `(i + 1) % n` neighbour (or the FIRST from
///   no selection); an EMPTY player gang is a no-op.
/// - [`ActIntent::SelectPrev`] sets the WRAPPING `(i + n - 1) % n` neighbour (or the LAST
///   from no selection); an EMPTY player gang is a no-op.
///
/// Both are resolved directly in the drain via [`cycle_player_selection`] over the
/// [`SelectionCycleReads`] bundle, writing [`SelectedShooter`] only on a real change
/// (change-detection hygiene — the `set_selection` precedent).
///
/// The GTW-735 DIRECT-select intent picks an actor BY ENTITY token, under the SAME
/// player-faction gate the left-click SELECT clause ([`decide_left_click`](crate::decide_left_click)) enforces:
///
/// - [`ActIntent::Select`] sets [`SelectedShooter`] to the carried entity ONLY when it is a LIVE
///   player-faction ganger (`SelectionCycleReads::select_target` — the SAME `faction == player`
///   gate `decide_left_click`'s clause 2 uses); an ENEMY, a non-ganger, or a DEAD / despawned
///   token is REFUSED (fail-closed — the selection is left untouched, no panic), and it writes
///   only on a real change (the same `set_selection` hygiene the cycle arms use). It has NO local
///   producer — the keyboard / UI / mouse surfaces are UNCHANGED; its producer is the GTW-694 T4
///   network inject path.
///
/// With NO [`SelectedShooter`] the posture cycle intents are no-ops (nothing to act on); a
/// cycle intent for a selected entity that lacks the relevant component is skipped
/// (fail-closed, no panic) via the query lookup. Param-only (`bevy-traps.md` #7):
/// [`ResMut`] over the queue + the presenter [`ActiveLevel`] +
/// [`ViewMode`](gdtf_battle_presenter::ViewMode) (the GTW-521 full-view toggle target), a
/// read-only `actors` [`Query`], the [`ActWriters`] message-writer bundle (`bevy-traps.md`
/// #4 — buffered messages), and the [`SelectionCycleReads`] read bundle for the Prev/Next
/// cycle.
/// Registered `.after` the intent WRITERS (`bevy-traps.md` #3) so it drains the same
/// update's pushes.
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-727 adds the playback gate to a signature that was already at the ceiling; \
              every param is a distinct, independently-borrowed access the drain genuinely \
              needs, and the two multi-read groups are ALREADY bundled (`ActWriters` / \
              `SelectionCycleReads`). Bundling the gate into one of those would attach it to \
              an unrelated concern"
)]
pub fn dispatch_act_intents(
    gate: PlaybackGate,
    mut pending: ResMut<PendingActIntent>,
    mut selected: ResMut<SelectedShooter>,
    mut active_level: ResMut<ActiveLevel>,
    mut view_mode: ResMut<ViewMode>,
    actors: Query<(&Stance, &Facing, &Aiming)>,
    mut acts: ActWriters,
    cycle_reads: SelectionCycleReads,
) {
    // GTW-727 C23: the queue is ALWAYS drained, even while the presenter is still catching
    // up. It must be: `PendingActIntent::drain` is a `mem::take`, so a skipped drain does not
    // defer the intents — it accumulates them, and the instant the gate opens the whole
    // backlog fires at once. That is "acting on unshown information", merely delayed.
    //
    // So the gate is applied PER ARM instead, and it DISCARDS. An intent the player formed
    // against a screen that was already out of date is stale whenever it eventually executes;
    // holding it only makes it stale AND surprising. The arms that survive a closed gate are
    // the ones that change nothing about the world — the presenter-owned view controls.
    let gate_open = gate.is_open();
    for intent in pending.drain() {
        if !gate_open && intent.needs_caught_up() {
            continue;
        }
        match intent {
            ActIntent::SelectionClear => {
                if selected.is_some() {
                    *selected = SelectedShooter::cleared();
                }
            }
            ActIntent::LevelUp => {
                let next = step_level(**active_level, LevelStep::Up);
                if next != **active_level {
                    *active_level = ActiveLevel::new(next);
                }
            }
            ActIntent::LevelDown => {
                let next = step_level(**active_level, LevelStep::Down);
                if next != **active_level {
                    *active_level = ActiveLevel::new(next);
                }
            }
            ActIntent::ToggleFullView => {
                // GTW-521: flip the presenter-owned ViewMode between DownToActive and FullView,
                // via the presenter's own ViewMode::toggled (GTW-577 C8 — the ONE flip, shared
                // with the editor's toggle surfaces). A no-act presenter-view intent like
                // LevelUp/LevelDown — it does NOT touch ActiveLevel (C5). Assigning through the
                // ResMut marks it changed, so the presenter's terrain draw + ganger visibility
                // re-run on the flip (C3). Always a real change (the two variants differ), so no
                // change-guard is needed.
                let flipped = view_mode.toggled();
                *view_mode = flipped;
            }
            ActIntent::StanceCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((stance, ..)) = actors.get(actor) else {
                    continue;
                };
                let next: StanceKind = cycle::next_stance(**stance);
                acts.stance.write(SetStanceRequested::new(actor, next));
            }
            ActIntent::SetStance(kind) => {
                // Direct set (GTW-267): emit the carried posture for the selection. No
                // need to read the current stance — the action-bar toggle named the exact
                // target posture. With no selection there is nothing to act on.
                let Some(actor) = **selected else { continue };
                acts.stance.write(SetStanceRequested::new(actor, kind));
            }
            ActIntent::AimToggle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, _, aiming)) = actors.get(actor) else {
                    continue;
                };
                acts.aiming
                    .write(SetAimingRequested::new(actor, AimRequest::new(!**aiming)));
            }
            ActIntent::FacingCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, facing, ..)) = actors.get(actor) else {
                    continue;
                };
                let next = cycle::next_facing(**facing);
                acts.facing.write(SetFacingRequested::new(actor, next));
            }
            ActIntent::Fire(request) => {
                acts.fire.write(request);
            }
            ActIntent::Move(request) => {
                acts.movement.write(request);
            }
            ActIntent::Turn(request) => {
                acts.facing.write(request);
            }
            ActIntent::Reload => {
                // Emit a reload for the selection only — mirror the SetStance arm
                // (GTW-275). With no selection there is nothing to reload.
                let Some(actor) = **selected else { continue };
                acts.reload.write(ReloadRequested::new(actor));
            }
            ActIntent::EndTurn => {
                // Emit the fieldless turn signal (GTW-309). A GLOBAL act like SelectionClear
                // / LevelUp — no selection needed; the sim's ActiveFaction tracks whose turn
                // is ending, so the drain just writes the unit message unconditionally.
                acts.end_turn.write(EndTurnRequested);
            }
            ActIntent::SelectNext => {
                cycle_selection(&mut selected, &cycle_reads, CycleDirection::Next);
            }
            ActIntent::SelectPrev => {
                cycle_selection(&mut selected, &cycle_reads, CycleDirection::Prev);
            }
            ActIntent::Select(actor) => {
                // GTW-735 DIRECT actor selection: apply the SAME player-faction gate the
                // left-click SELECT clause (`decide_left_click`) enforces via `select_target`
                // (`faction == player`), plus the `set_selection` change-detection hygiene — write
                // SelectedShooter ONLY on a real change. A dead / despawned token, a non-ganger, an
                // enemy, or a pre-battle absent PlayerFaction all refuse the selection
                // (fail-closed, no panic). The drain stays the SOLE writer of SelectedShooter for
                // intent-driven selection. `*selected` is the whole SelectedShooter (one deref for
                // `ResMut`); assigning through it trips change-detection for the highlight + sync.
                if let Some(next) = cycle_reads.select_target(actor)
                    && *selected != next
                {
                    *selected = next;
                }
            }
        }
    }
}

/// Steps the [`SelectedShooter`] to the `direction` neighbour in the shared `(z, y, x)`
/// player-gang order (GTW-458), writing only on a real change.
///
/// Collects the deterministic player-faction order from `reads`
/// ([`SelectionCycleReads::ordered_player_gangers`]) and picks the wrapping neighbour with
/// [`cycle_player_selection`] (enemies excluded; first/last-from-`None`; empty gang →
/// no-op). Mirrors the `set_selection` change-detection hygiene — it writes the new
/// selection ONLY when it differs from the current one, so a redundant cycle on a one-ganger
/// gang does not spuriously trip `Changed<SelectedShooter>`.
fn cycle_selection(
    selected: &mut ResMut<SelectedShooter>,
    reads: &SelectionCycleReads,
    direction: CycleDirection,
) {
    let ordered = reads.ordered_player_gangers();
    // `***selected` reads the inner `Option<Entity>` through the Deref chain
    // (`&mut ResMut` → `ResMut` → `SelectedShooter` → `Option<Entity>`).
    let Some(next) = cycle_player_selection(&ordered, ***selected, direction) else {
        // Empty player gang — nothing to cycle to; leave the selection untouched.
        return;
    };
    let next = SelectedShooter::new(next);
    // `**selected` is the whole `SelectedShooter` (one deref for `&mut`, one for `ResMut`'s
    // `DerefMut`). Write only on a real change (the `set_selection` change-detection hygiene).
    if **selected != next {
        **selected = next;
    }
}

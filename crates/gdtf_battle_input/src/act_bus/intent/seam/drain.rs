//! Drain pending intents into sim messages and selection updates.

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

#[expect(
    clippy::too_many_arguments,
    reason = "each param is a distinct access; ActWriters and SelectionCycleReads already bundle the multi-reads"
)]
/// Dispatch each pending intent when the playback gate allows it.
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
                let Some(actor) = **selected else { continue };
                acts.reload.write(ReloadRequested::new(actor));
            }
            ActIntent::EndTurn => {
                acts.end_turn.write(EndTurnRequested);
            }
            ActIntent::SelectNext => {
                cycle_selection(&mut selected, &cycle_reads, CycleDirection::Next);
            }
            ActIntent::SelectPrev => {
                cycle_selection(&mut selected, &cycle_reads, CycleDirection::Prev);
            }
            ActIntent::Select(actor) => {
                if let Some(next) = cycle_reads.select_target(actor)
                    && *selected != next
                {
                    *selected = next;
                }
            }
        }
    }
}

fn cycle_selection(
    selected: &mut ResMut<SelectedShooter>,
    reads: &SelectionCycleReads,
    direction: CycleDirection,
) {
    let ordered = reads.ordered_player_gangers();
    let Some(next) = cycle_player_selection(&ordered, ***selected, direction) else {
        return;
    };
    let next = SelectedShooter::new(next);
    if **selected != next {
        **selected = next;
    }
}

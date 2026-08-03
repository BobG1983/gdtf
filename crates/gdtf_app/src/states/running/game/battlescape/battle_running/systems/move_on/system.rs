use bevy::prelude::*;
use gdtf_battle_presenter::{PendingImpact, PlaybackGate, Played, ShotProjectile};
use gdtf_battle_sim::shot_fired::ShotFired;

use super::phase::{EndPhase, EndTransition, SeenBusy};
use crate::states::BattleScapeState;

#[expect(
    clippy::too_many_arguments,
    reason = "the gate reads four independent facts before it may leave — the clock (its \
              backstop), the persisted phase, the playback cursor, and the FX pipeline (two \
              disjoint marker queries) — plus the shown-shot classifier and the two writes. \
              Each is a distinct Bevy SystemParam; bundling them would hide exactly the \
              inputs this decision is documented on"
)]
pub(in crate::states::running::game::battlescape::battle_running) fn move_on(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<NextState<BattleScapeState>>,
    transition: Option<ResMut<EndTransition>>,
    playback: PlaybackGate,
    projectiles: Query<(), With<ShotProjectile>>,
    pending: Query<(), With<PendingImpact>>,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    let pipeline_busy = !projectiles.is_empty() || !pending.is_empty();

    let deciding_shot_shown = shots.read().next().is_some();

    let mut transition = transition;
    let phase = if let Some(existing) = transition.as_deref_mut() {
        existing.tick(time.delta());
        existing.phase()
    } else {
        let initial = if deciding_shot_shown {
            EndPhase::AwaitingDecidingShot {
                seen_busy: SeenBusy::new(pipeline_busy),
            }
        } else {
            EndPhase::NoDecidingShot
        };
        commands.insert_resource(EndTransition::new(initial));
        initial
    };

    if transition
        .as_deref()
        .is_some_and(EndTransition::backstop_elapsed)
    {
        state.set(BattleScapeState::AnimateOut);
        return;
    }

    if !playback.is_open() {
        if pipeline_busy && let Some(existing) = transition.as_deref_mut() {
            existing.mark_seen_busy();
        }
        return;
    }

    match phase {
        EndPhase::NoDecidingShot => {
            state.set(BattleScapeState::AnimateOut);
        }
        EndPhase::AwaitingDecidingShot { seen_busy } => {
            if pipeline_busy {
                if let Some(existing) = transition.as_deref_mut() {
                    existing.mark_seen_busy();
                }
            } else if *seen_busy {
                state.set(BattleScapeState::AnimateOut);
            }
        }
    }
}

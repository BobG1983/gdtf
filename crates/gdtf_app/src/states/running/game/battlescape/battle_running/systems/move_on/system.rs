use bevy::prelude::*;

use super::{
    phase::{EndPhase, EndTransition, SeenBusy},
    shown::ShotPresentation,
};
use crate::states::BattleScapeState;

pub(in crate::states::running::game::battlescape::battle_running) fn move_on(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<NextState<BattleScapeState>>,
    transition: Option<ResMut<EndTransition>>,
    mut presentation: ShotPresentation,
) {
    let pipeline_busy = presentation.fx_busy();

    let deciding_shot_shown = *presentation.shot_shown();

    let mut transition = transition;
    let phase = if let Some(existing) = transition.as_deref_mut() {
        existing.tick(time.delta());
        existing.phase()
    } else {
        let initial = if deciding_shot_shown {
            EndPhase::AwaitingDecidingShot {
                seen_busy: SeenBusy::new(*pipeline_busy),
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

    if !*presentation.caught_up() {
        if *pipeline_busy && let Some(existing) = transition.as_deref_mut() {
            existing.mark_seen_busy();
        }
        return;
    }

    match phase {
        EndPhase::NoDecidingShot => {
            state.set(BattleScapeState::AnimateOut);
        }
        EndPhase::AwaitingDecidingShot { seen_busy } => {
            if *pipeline_busy {
                if let Some(existing) = transition.as_deref_mut() {
                    existing.mark_seen_busy();
                }
            } else if *seen_busy {
                state.set(BattleScapeState::AnimateOut);
            }
        }
    }
}

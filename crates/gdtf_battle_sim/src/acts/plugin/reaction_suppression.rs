use bevy::prelude::{App, IntoScheduleConfigs, Update};

use crate::{
    acts::{fire::dispatch_fire, movement::advance_walk},
    occupancy_sync::SimSystems,
    reaction::{reaction_trigger, reset_reactions_used},
    suppression::{apply_suppression, reset_suppression, suppression_auto_stance},
    turn::dispatch_end_turn,
};

pub(super) fn wire_reaction_suppression(app: &mut App) {
    app.add_systems(
        Update,
        reaction_trigger
            .before(dispatch_fire)
            .before(advance_walk)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        reset_reactions_used
            .after(dispatch_end_turn)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        apply_suppression
            .after(dispatch_fire)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        suppression_auto_stance
            .after(apply_suppression)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        reset_suppression
            .after(dispatch_end_turn)
            .in_set(SimSystems::Simulate),
    );
}

use bevy::prelude::{App, IntoScheduleConfigs, Update, resource_exists};

use crate::{
    acts::{
        dispatch_set_aiming, dispatch_set_stance, downed::dispatch_stabilize_downed,
        fire::dispatch_fire, injury::apply_injury, melee::dispatch_melee, movement::dispatch_move,
        reload::dispatch_reload,
    },
    ai::enemy_ai_turn,
    effects::{
        bleed::{enemy_phase_started, mark_downed_bleeding, tick_bleed},
        dot::{apply_dot, tick_dot},
        fields::tick_fields,
        on_death::resolve_on_death,
    },
    falls::apply_falls,
    occupancy::project_path_blocking,
    occupancy_sync::SimSystems,
    turn::{ActiveFaction, dispatch_end_turn},
};

pub(super) fn wire_turn_clocks(app: &mut App) {
    app.add_systems(
        Update,
        dispatch_end_turn
            .run_if(resource_exists::<ActiveFaction>)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        enemy_ai_turn
            .run_if(resource_exists::<ActiveFaction>)
            .after(dispatch_end_turn)
            .after(project_path_blocking)
            .before(dispatch_fire)
            .before(dispatch_melee)
            .before(dispatch_reload)
            .before(dispatch_set_aiming)
            .before(dispatch_set_stance)
            .before(dispatch_move)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        tick_bleed
            .after(dispatch_end_turn)
            .before(dispatch_fire)
            .before(dispatch_move)
            .before(dispatch_melee)
            .run_if(enemy_phase_started)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        mark_downed_bleeding
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(apply_falls)
            .after(tick_bleed)
            .before(dispatch_stabilize_downed)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        apply_injury
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(apply_falls)
            .in_set(SimSystems::Simulate),
    );
    wire_clocks(app);
}

fn wire_clocks(app: &mut App) {
    wire_dot(app);
    wire_on_death(app);
}

fn wire_on_death(app: &mut App) {
    app.add_systems(
        Update,
        resolve_on_death
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(tick_bleed)
            .after(tick_dot)
            .after(tick_fields)
            .after(apply_falls)
            .run_if(resource_exists::<crate::occupancy::OccupancyGrid>)
            .run_if(resource_exists::<crate::effects::fields::FieldRegistry>)
            .run_if(resource_exists::<crate::effects::on_death::CoverOnDeathRegistry>)
            .in_set(SimSystems::Simulate),
    );
}

fn wire_dot(app: &mut App) {
    app.add_systems(
        Update,
        apply_dot.after(dispatch_fire).in_set(SimSystems::Simulate),
    )
    .add_systems(
        Update,
        tick_dot
            .after(dispatch_end_turn)
            .before(dispatch_fire)
            .before(dispatch_move)
            .before(dispatch_melee)
            .run_if(enemy_phase_started)
            .in_set(SimSystems::Simulate),
    )
    .add_systems(
        Update,
        tick_fields
            .after(dispatch_end_turn)
            .after(tick_dot)
            .before(dispatch_fire)
            .before(dispatch_move)
            .before(dispatch_melee)
            .run_if(enemy_phase_started)
            .run_if(resource_exists::<crate::effects::fields::FieldRegistry>)
            .in_set(SimSystems::Simulate),
    );
}

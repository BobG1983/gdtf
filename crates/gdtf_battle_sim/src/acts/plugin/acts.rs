use bevy::prelude::{App, IntoScheduleConfigs, Update};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        enter_emplacement::{dispatch_enter_emplacement, dispatch_exit_emplacement},
        fire::dispatch_fire,
        melee::dispatch_melee,
        movement::{advance_walk, dispatch_move},
        open_door::dispatch_open_door,
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        reload::dispatch_reload,
        shove::dispatch_shove,
        throw_grenade::dispatch_throw_grenade,
    },
    equipment::attachments::apply_pending_attachments,
    occupancy::project_path_blocking,
    occupancy_sync::{SimSystems, sync_destroyed_cover},
};

pub(super) fn wire_acts(app: &mut App) {
    app.add_systems(
        Update,
        (
            dispatch_fire,
            dispatch_set_aiming,
            dispatch_set_stance,
            dispatch_set_facing,
            dispatch_stabilize_downed,
            dispatch_execute_downed,
            dispatch_reload,
            dispatch_melee,
            dispatch_open_door,
            dispatch_enter_emplacement,
            dispatch_exit_emplacement,
        )
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        dispatch_move
            .after(sync_destroyed_cover)
            .after(project_path_blocking)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        advance_walk
            .after(dispatch_move)
            .after(sync_destroyed_cover)
            .after(project_path_blocking)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(
        Update,
        dispatch_shove
            .after(dispatch_fire)
            .after(dispatch_melee)
            .in_set(SimSystems::Simulate),
    );
    app.add_systems(Update, dispatch_throw_grenade.in_set(SimSystems::Simulate));
    app.add_systems(
        Update,
        apply_pending_attachments
            .before(dispatch_fire)
            .in_set(SimSystems::Simulate),
    );
}

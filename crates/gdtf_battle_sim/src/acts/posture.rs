use bevy::prelude::{MessageReader, Query, Res};

use crate::{
    acts::request::{SetAimingRequested, SetFacingRequested, SetStanceRequested},
    ganger::{Aiming, Facing, Stance, Tu},
    posture::{set_aiming, set_facing, set_stance},
    tuning::CombatTuning,
};

pub fn dispatch_set_aiming(
    mut requests: MessageReader<SetAimingRequested>,
    mut actors: Query<&'static mut Aiming>,
) {
    for request in requests.read() {
        let Ok(mut aiming) = actors.get_mut(request.actor) else {
            continue;
        };
        set_aiming(&mut aiming, Aiming::new(*request.aim));
    }
}

pub fn dispatch_set_stance(
    mut requests: MessageReader<SetStanceRequested>,
    mut actors: Query<(&'static mut Stance, &'static mut Tu)>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((mut stance, mut tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        set_stance(
            &mut stance,
            &mut tu,
            request.stance,
            &tuning.stance_change_tu,
        );
    }
}

pub fn dispatch_set_facing(
    mut requests: MessageReader<SetFacingRequested>,
    mut actors: Query<(&'static mut Facing, &'static mut Tu)>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((mut facing, mut tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        set_facing(&mut facing, &mut tu, request.facing, &tuning.turn_tu);
    }
}

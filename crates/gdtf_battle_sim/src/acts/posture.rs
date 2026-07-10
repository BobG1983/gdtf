//! The **posture** dispatch systems — drain each buffered posture `*Requested` message
//! and run the landed posture verb once per message (E10.2 AC4).
//!
//! No act logic is reimplemented: each system REUSES the landed
//! [`set_aiming`](crate::posture::set_aiming) / [`set_stance`](crate::posture::set_stance)
//! / [`set_facing`](crate::posture::set_facing) verb verbatim (with its TU semantics), and
//! fetches the actor's components via Bevy queries (`bevy-traps.md` #7 — no `&mut World`).

use bevy::prelude::{MessageReader, Query, Res};

use crate::{
    acts::request::{SetAimingRequested, SetFacingRequested, SetStanceRequested},
    ganger::{Aiming, Facing, Stance, Tu},
    posture::{set_aiming, set_facing, set_stance},
    tuning::CombatTuning,
};

/// **Dispatch** buffered [`SetAimingRequested`] messages — drain each and run the landed
/// [`set_aiming`] verb once per message (E10.2 AC4).
///
/// Queries the actor's [`Aiming`] component and calls [`set_aiming`] (charges NO TU —
/// toggling aim is free). REUSES the landed verb verbatim. A message for an actor without
/// an [`Aiming`] component is skipped (fail-closed, no panic).
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

/// **Dispatch** buffered [`SetStanceRequested`] messages — drain each and run the landed
/// [`set_stance`] verb once per message (E10.2 AC4).
///
/// Queries the actor's [`Stance`] + [`Tu`] components plus the [`CombatTuning`] resource
/// and calls [`set_stance`] (charges [`StanceChangeTu`](crate::tuning::StanceChangeTu)
/// only on a real change — a no-op, no charge, when the actor already holds the requested
/// stance). REUSES the landed verb verbatim. A message for an actor missing either
/// component is skipped (fail-closed, no panic).
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

/// **Dispatch** buffered [`SetFacingRequested`] messages — drain each and run the landed
/// [`set_facing`] verb once per message (E10.2 AC4).
///
/// Queries the actor's [`Facing`] + [`Tu`] components plus the [`CombatTuning`] resource
/// and calls [`set_facing`] (charges [`TurnTu`](crate::tuning::TurnTu) only on a real
/// turn — a no-op, no charge, when the actor already faces the requested direction).
/// REUSES the landed verb verbatim. A message for an actor missing either component is
/// skipped (fail-closed, no panic).
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

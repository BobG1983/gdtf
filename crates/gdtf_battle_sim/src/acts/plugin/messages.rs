use bevy::prelude::App;

use crate::{
    acts::{
        fire::FireDeclaration,
        injury::InjuryInflicted,
        movement::{MoveRejected, MovementOccurred, ReactionShotFired},
        reload::ReloadResult,
        request::{
            EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
            ExitEmplacementRequested, FireRequested, MeleeRequested, MeleeResolved, MeleeStruck,
            MoveRequested, OpenDoorRequested, ReloadRequested, SetAimingRequested,
            SetFacingRequested, SetStanceRequested, ShoveRequested, StabilizeDownedRequested,
            ThrowGrenadeRequested, ThrowResolved,
        },
    },
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotApplied, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    terrain::{emplacement::SetEmplacement, openable::SetOpenable},
    turn::TurnStarted,
};

pub(super) fn register_messages(app: &mut App) {
    app.add_message::<FireRequested>()
        .add_message::<SetAimingRequested>()
        .add_message::<SetStanceRequested>()
        .add_message::<SetFacingRequested>()
        .add_message::<StabilizeDownedRequested>()
        .add_message::<ExecuteDownedRequested>()
        .add_message::<MoveRequested>()
        .add_message::<ReloadRequested>()
        .add_message::<OpenDoorRequested>()
        .add_message::<SetOpenable>()
        .add_message::<EnterEmplacementRequested>()
        .add_message::<ExitEmplacementRequested>()
        .add_message::<ThrowGrenadeRequested>()
        .add_message::<ThrowResolved>()
        .add_message::<SetEmplacement>()
        .add_message::<MeleeRequested>()
        .add_message::<MeleeResolved>()
        .add_message::<MeleeStruck>()
        .add_message::<ShoveRequested>()
        .add_message::<FallOccurred>()
        .add_message::<EndTurnRequested>()
        .add_message::<ShotFired>()
        .add_message::<CoverDestroyed>()
        .add_message::<SlabDestroyed>()
        .add_message::<GroundAccrued>()
        .add_message::<ReloadResult>()
        .add_message::<FireDeclaration>()
        .add_message::<MovementOccurred>()
        .add_message::<TurnStarted>()
        .add_message::<MoveRejected>()
        .add_message::<Bleeding>()
        .add_message::<crate::armor_wear::ArmorBroken>()
        .add_message::<BleedStarted>()
        .add_message::<InjuryInflicted>()
        .add_message::<DotApplied>()
        .add_message::<DotAfflicted>()
        .add_message::<DotTicked>()
        .add_message::<FieldTicked>()
        .add_message::<FieldAfflicted>()
        .add_message::<OnDeathOccurred>()
        .add_message::<ReactionShotFired>()
        .add_message::<InterruptDeclared>()
        .add_message::<SuppressionApplied>();
}

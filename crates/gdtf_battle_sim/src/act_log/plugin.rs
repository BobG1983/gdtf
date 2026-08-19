//! Wire act-log recording into the sim schedule.

use bevy::prelude::*;

use super::{log::ActLog, record::record_acts};
use crate::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::{SimSystems, TerrainPieceDestroyed},
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

/// Register messages and the [`record_acts`] system when [`ActLog`] is present.
pub fn wire_act_log(app: &mut App) {
    app.add_message::<InterruptDeclared>()
        .add_message::<TurnStarted>()
        .add_message::<MovementOccurred>()
        .add_message::<MoveRejected>()
        .add_message::<FireDeclaration>()
        .add_message::<ShotFired>()
        .add_message::<ReloadResult>()
        .add_message::<InjuryInflicted>()
        .add_message::<FallOccurred>()
        .add_message::<MeleeStruck>()
        .add_message::<OnDeathOccurred>()
        .add_message::<SuppressionApplied>()
        .add_message::<ArmorBroken>()
        .add_message::<DotAfflicted>()
        .add_message::<DotTicked>()
        .add_message::<FieldAfflicted>()
        .add_message::<FieldTicked>()
        .add_message::<BleedStarted>()
        .add_message::<Bleeding>()
        .add_message::<TerrainPieceDestroyed>()
        .add_message::<MeleeResolved>()
        .add_message::<ThrowResolved>()
        .add_systems(
            Update,
            record_acts
                .in_set(SimSystems::Record)
                .run_if(resource_exists::<ActLog>),
        );
}

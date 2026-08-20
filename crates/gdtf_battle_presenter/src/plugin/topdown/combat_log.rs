use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::*,
};
use gdtf_battle_sim::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeStruck, MoveCompleted, MoveRejected, ReloadResult,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::BleedStarted, dot::DotAfflicted, fields::FieldAfflicted, on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    prelude::BattleInProgress,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

use crate::{
    CombatLogEvent, CombatLogSourceAppExt, CombatLogSystems, ShotImpactResolved,
    forward_turn_started,
};

pub(super) fn register_combat_log_forwarders(app: &mut App) {
    app.add_message::<CombatLogEvent>();

    app.add_combat_log_source::<FireDeclaration>()
        .add_combat_log_source::<MoveCompleted>()
        .add_combat_log_source::<MoveRejected>()
        .add_live_combat_log_source::<ShotImpactResolved>()
        .add_combat_log_source::<ReloadResult>()
        .add_combat_log_source::<InjuryInflicted>()
        .add_combat_log_source::<FallOccurred>()
        .add_combat_log_source::<MeleeStruck>()
        .add_combat_log_source::<OnDeathOccurred>()
        .add_combat_log_source::<SuppressionApplied>()
        .add_combat_log_source::<ArmorBroken>()
        .add_combat_log_source::<DotAfflicted>()
        .add_combat_log_source::<FieldAfflicted>()
        .add_combat_log_source::<BleedStarted>();

    app.add_systems(
        Update,
        forward_turn_started
            .in_set(CombatLogSystems::Forward)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<Messages<TurnStarted>>),
            ),
    );
}

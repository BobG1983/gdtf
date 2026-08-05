//! Emit melee connect messages: resolved, struck, injury, shove, death.

use bevy::prelude::Entity;

use super::{MeleeOutcomes, queries::MeleeTargetQuery, snapshot::AttackerSnapshot};
use crate::{
    acts::{
        InjuryInflicted,
        request::{MeleeResolved, MeleeStruck, ShoveRequested},
    },
    armor_wear::ArmorWearOutcome,
    effects::on_death::OnDeathOccurred,
    ganger::LifeState,
    melee::MeleeStrike,
    metric::CellLevel,
};

/// Write outcome messages for a connected melee strike.
pub(super) fn emit_connect_signals(
    attacker: &AttackerSnapshot<'_>,
    target_entity: Entity,
    at: CellLevel,
    strike: MeleeStrike,
    targets: &MeleeTargetQuery,
    outcomes: &mut MeleeOutcomes,
) {
    outcomes
        .resolved
        .write(MeleeResolved::new(at, attacker.strike_damage_type));

    outcomes.facts.struck.write(MeleeStruck::new(
        attacker.entity,
        target_entity,
        strike.hp_damage,
    ));

    if let ArmorWearOutcome::Broke(broken) = strike.wear {
        outcomes.facts.breaks.write(broken);
    }

    if let Some(rolled) = strike.injury {
        outcomes
            .facts
            .injuries
            .write(InjuryInflicted::from_rolled(target_entity, rolled));
    }

    if targets
        .get(target_entity)
        .is_ok_and(|(_, _, &life, ..)| life == LifeState::Dead)
    {
        outcomes
            .deaths
            .write(OnDeathOccurred::new(target_entity, at));
    }

    if *attacker.shove {
        outcomes
            .shoves
            .write(ShoveRequested::new_weapon(attacker.entity, target_entity));
    }
}

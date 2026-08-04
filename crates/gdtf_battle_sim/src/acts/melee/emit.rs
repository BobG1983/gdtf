//! Emit melee connect messages: resolved, struck, injury, shove, death.

use bevy::prelude::{Entity, MessageWriter};

use super::{MeleeFacts, queries::MeleeTargetQuery, snapshot::AttackerSnapshot};
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

/// Writers used after a successful melee connect.
pub(super) struct MeleeConnectSignals<'a, 'r, 'f, 's, 'd> {
    pub(super) resolved: &'a mut MessageWriter<'r, MeleeResolved>,
    pub(super) facts:    &'a mut MeleeFacts<'f>,
    pub(super) shoves:   &'a mut MessageWriter<'s, ShoveRequested>,
    pub(super) deaths:   &'a mut MessageWriter<'d, OnDeathOccurred>,
}

/// Write outcome messages for a connected melee strike.
pub(super) fn emit_connect_signals(
    attacker: &AttackerSnapshot<'_>,
    target_entity: Entity,
    at: CellLevel,
    strike: MeleeStrike,
    targets: &MeleeTargetQuery,
    signals: MeleeConnectSignals<'_, '_, '_, '_, '_>,
) {
    signals
        .resolved
        .write(MeleeResolved::new(at, attacker.strike_damage_type));

    signals.facts.struck.write(MeleeStruck::new(
        attacker.entity,
        target_entity,
        strike.hp_damage,
    ));

    if let ArmorWearOutcome::Broke(broken) = strike.wear {
        signals.facts.breaks.write(broken);
    }

    if let Some(rolled) = strike.injury {
        signals
            .facts
            .injuries
            .write(InjuryInflicted::from_rolled(target_entity, rolled));
    }

    if targets
        .get(target_entity)
        .is_ok_and(|(_, _, &life, ..)| life == LifeState::Dead)
    {
        signals
            .deaths
            .write(OnDeathOccurred::new(target_entity, at));
    }

    if *attacker.shove {
        signals
            .shoves
            .write(ShoveRequested::new_weapon(attacker.entity, target_entity));
    }
}

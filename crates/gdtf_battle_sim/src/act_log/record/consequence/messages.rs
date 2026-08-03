use bevy::prelude::Entity;

use super::super::sources::{ConsequenceMessages, ProvenanceSources};
use crate::act_log::{ActDeed, ActLog, RecordedAct};

pub(in crate::act_log::record) fn record_consequence_messages(
    log: &mut ActLog,
    messages: &mut ConsequenceMessages,
    provenance: &ProvenanceSources,
) {
    for reload in messages.reloads.read() {
        log.append(RecordedAct::new(
            reload.actor,
            provenance.of(reload.actor),
            ActDeed::Reloaded {
                outcome: reload.outcome,
            },
        ));
    }
    for injury in messages.injuries.read() {
        log.append(RecordedAct::new(
            injury.target,
            provenance.of(injury.target),
            ActDeed::Injured {
                injury: Box::new(injury.clone()),
            },
        ));
    }
    for fall in messages.falls.read() {
        log.append(RecordedAct::new(
            fall.ganger,
            provenance.of(fall.ganger),
            ActDeed::Fell {
                from_level: fall.from_level,
                to_level:   fall.to_level,
                storeys:    fall.storeys,
            },
        ));
    }
    for strike in messages.strikes.read() {
        log.append(RecordedAct::new(
            strike.attacker,
            provenance.of(strike.attacker),
            ActDeed::Struck {
                target:    strike.target,
                hp_damage: strike.hp_damage,
            },
        ));
    }
    for death in messages.deaths.read() {
        log.append(RecordedAct::new(
            death.entity,
            provenance.of(death.entity),
            ActDeed::DiedAt { at: death.at },
        ));
    }
    for suppression in messages.suppressions.read() {
        log.append(RecordedAct::new(
            suppression.ganger,
            provenance.of(suppression.ganger),
            ActDeed::Suppressed { at: suppression.at },
        ));
    }
    for broken in messages.armor_breaks.read() {
        log.append(RecordedAct::new(
            broken.ganger,
            provenance.of(broken.ganger),
            ActDeed::ArmorBroke { part: broken.part },
        ));
    }
    record_afflictions(log, messages);
}

fn record_afflictions(log: &mut ActLog, messages: &mut ConsequenceMessages) {
    for dot in messages.dots.read() {
        log.append(RecordedAct::new(
            dot.ganger,
            ProvenanceSources::clock(),
            ActDeed::DotStarted {
                per_turn: dot.per_turn,
            },
        ));
    }
    for field in messages.fields.read() {
        log.append(RecordedAct::new(
            field.occupant,
            ProvenanceSources::clock(),
            ActDeed::FieldStarted { at: field.at },
        ));
    }
    for bleed in messages.bleeds.read() {
        log.append(RecordedAct::new(
            bleed.ganger,
            ProvenanceSources::clock(),
            ActDeed::BleedStarted,
        ));
    }
    for bleed in messages.bleed_ticks.read() {
        log.append(RecordedAct::new(
            bleed.ganger,
            ProvenanceSources::clock(),
            ActDeed::Bled,
        ));
    }
    for dot in messages.dot_ticks.read() {
        log.append(RecordedAct::new(
            dot.ganger,
            ProvenanceSources::clock(),
            ActDeed::DotTicked {
                at:     dot.at,
                amount: dot.amount,
            },
        ));
    }
    for field in messages.field_ticks.read() {
        log.append(RecordedAct::new(
            field.occupant,
            ProvenanceSources::clock(),
            ActDeed::FieldTicked {
                at:     field.at,
                amount: field.amount,
            },
        ));
    }
    for smashed in messages.cover_smashed.read() {
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ProvenanceSources::clock(),
            ActDeed::CoverSmashed { at: smashed.at },
        ));
    }
    for strike in messages.melee_landed.read() {
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ProvenanceSources::clock(),
            ActDeed::MeleeLanded {
                at:     strike.at,
                damage: strike.damage,
            },
        ));
    }
    for landed in messages.throw_landed.read() {
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ProvenanceSources::clock(),
            ActDeed::ThrowLanded {
                at:     landed.at,
                damage: landed.damage,
            },
        ));
    }
}

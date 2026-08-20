use bevy::prelude::Entity;

use super::super::sources::{ActObservation, ConsequenceMessages, ProvenanceSources};
use crate::act_log::{ActDeed, ActLog, RecordedAct};

pub(in crate::act_log::record) fn record_consequence_messages(
    log: &mut ActLog,
    messages: &mut ConsequenceMessages,
    provenance: &ProvenanceSources,
    seen: &ActObservation,
) {
    for reload in messages.reloads.read() {
        log.append(RecordedAct::new(
            reload.actor,
            provenance.of(reload.actor),
            ActDeed::Reloaded {
                outcome: reload.outcome,
            },
            seen.of_actor(reload.actor),
        ));
    }
    for injury in messages.injuries.read() {
        log.append(RecordedAct::new(
            injury.target,
            provenance.of(injury.target),
            ActDeed::Injured {
                injury: Box::new(injury.clone()),
            },
            seen.of_actor(injury.target),
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
            seen.of_actor(fall.ganger),
        ));
    }
    for strike in messages.strikes.read() {
        let landed: Vec<_> = seen.cell_of(strike.target).into_iter().collect();
        log.append(RecordedAct::new(
            strike.attacker,
            provenance.of(strike.attacker),
            ActDeed::Struck {
                target:    strike.target,
                hp_damage: strike.hp_damage,
            },
            seen.of_effect(&landed, strike.attacker),
        ));
    }
    for death in messages.deaths.read() {
        log.append(RecordedAct::new(
            death.entity,
            provenance.of(death.entity),
            ActDeed::DiedAt { at: death.at },
            seen.of_cell(death.at),
        ));
    }
    for suppression in messages.suppressions.read() {
        log.append(RecordedAct::new(
            suppression.ganger,
            provenance.of(suppression.ganger),
            ActDeed::Suppressed { at: suppression.at },
            seen.of_cell(suppression.at),
        ));
    }
    for broken in messages.armor_breaks.read() {
        log.append(RecordedAct::new(
            broken.ganger,
            provenance.of(broken.ganger),
            ActDeed::ArmorBroke { part: broken.part },
            seen.of_actor(broken.ganger),
        ));
    }
    record_afflictions(log, messages, seen);
}

fn record_afflictions(log: &mut ActLog, messages: &mut ConsequenceMessages, seen: &ActObservation) {
    for dot in messages.dots.read() {
        log.append(RecordedAct::new(
            dot.ganger,
            ProvenanceSources::clock(),
            ActDeed::DotStarted {
                per_turn: dot.per_turn,
            },
            seen.of_actor(dot.ganger),
        ));
    }
    for field in messages.fields.read() {
        log.append(RecordedAct::new(
            field.occupant,
            ProvenanceSources::clock(),
            ActDeed::FieldStarted { at: field.at },
            seen.of_cell(field.at),
        ));
    }
    for bleed in messages.bleeds.read() {
        log.append(RecordedAct::new(
            bleed.ganger,
            ProvenanceSources::clock(),
            ActDeed::BleedStarted,
            seen.of_actor(bleed.ganger),
        ));
    }
    for bleed in messages.bleed_ticks.read() {
        log.append(RecordedAct::new(
            bleed.ganger,
            ProvenanceSources::clock(),
            ActDeed::Bled,
            seen.of_actor(bleed.ganger),
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
            seen.of_cell(dot.at),
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
            seen.of_cell(field.at),
        ));
    }
    for smashed in messages.cover_smashed.read() {
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ProvenanceSources::clock(),
            ActDeed::TerrainPieceSmashed {
                at:   smashed.at,
                kind: smashed.kind,
            },
            seen.of_cell_unnamed(smashed.at),
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
            seen.of_cell_unnamed(strike.at),
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
            seen.of_cell_unnamed(landed.at),
        ));
    }
}

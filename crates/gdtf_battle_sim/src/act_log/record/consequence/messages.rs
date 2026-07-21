//! The message-sourced half of the CONSEQUENCE family — everything an act left behind
//! that a sim signal already announces (GTW-727 C9, family 5 of 6).

use bevy::prelude::Entity;

use super::super::sources::{ConsequenceMessages, ProvenanceSources};
use crate::act_log::{ActDeed, ActLog, RecordedAct};

/// Record every consequence signal emitted this tick, in a fixed source order.
///
/// The order below IS the intra-tick ordering guarantee for this family — a property of
/// this function's source code, not of the scheduler, which is what makes the whole log
/// reproducible run to run (`bevy-traps.md` #3).
///
/// The three affliction families record their once-per-span START facts only
/// ([`DotAfflicted`](crate::effects::dot::DotAfflicted) /
/// [`FieldAfflicted`](crate::effects::fields::FieldAfflicted) /
/// [`BleedStarted`](crate::effects::bleed::BleedStarted)); their per-tick drain signals
/// have no deed at all, exactly as they have no combat-log line. The per-tick HP those
/// drains take still reaches a consumer — through the vitals transition recorded beside
/// this function.
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

/// Record the once-per-span AFFLICTION starts and the per-round drains.
///
/// Split from the act-consequence half above at the family's own change-reason boundary: an
/// affliction is a CLOCK beat about a ganger rather than something an actor did to it, so
/// its provenance is fixed and its sources are the per-round tick signals rather than an
/// act's output.
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
    for smashed in messages.cover_smashed.read() {
        // Cover is not an entity, so the deed names the cell and the entry's actor is the
        // placeholder — the same convention the cover on-death signal already uses.
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

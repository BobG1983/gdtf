use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::Entity};

use super::sources::{FireSources, ProvenanceSources};
use crate::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    shot_fired::ShotFired,
};

pub(super) fn record_fire(
    log: &mut ActLog,
    fire: &mut FireSources,
    provenance: &ProvenanceSources,
) {
    let mut interrupts: HashMap<Entity, VecDeque<Entity>> = HashMap::default();
    for declared in fire.interrupts.read() {
        interrupts
            .entry(declared.reactor)
            .or_default()
            .push_back(declared.interrupted);
    }

    let rounds: Vec<ShotFired> = fire.rounds.read().cloned().collect();
    let mut next_round = 0_usize;

    for declaration in fire.declarations.read() {
        let shooter = declaration.shooter;
        let cause = interrupts
            .get_mut(&shooter)
            .and_then(VecDeque::pop_front)
            .map_or_else(
                || provenance.of(shooter),
                |interrupted| ActProvenance::Reaction { interrupted },
            );
        log.append(RecordedAct::new(
            shooter,
            cause,
            ActDeed::Fired {
                target: declaration.target,
                mode:   declaration.mode,
                rounds: declaration.rounds,
            },
        ));
        let claimed = usize::try_from(*declaration.rounds).unwrap_or(usize::MAX);
        for shot in rounds.iter().skip(next_round).take(claimed) {
            log.append(RecordedAct::new(
                shooter,
                cause,
                ActDeed::RoundResolved {
                    shot: Box::new(shot.clone()),
                },
            ));
        }
        next_round = next_round.saturating_add(claimed);
    }
}

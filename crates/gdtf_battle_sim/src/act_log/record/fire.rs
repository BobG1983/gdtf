use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::Entity};

use super::sources::{ActObservation, FireSources, ProvenanceSources};
use crate::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct},
    march::cells_crossed,
    metric::CellLevel,
    shot_fired::ShotFired,
};

pub(super) fn record_fire(
    log: &mut ActLog,
    fire: &mut FireSources,
    provenance: &ProvenanceSources,
    seen: &ActObservation,
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
        let claimed = usize::try_from(*declaration.rounds).unwrap_or(usize::MAX);
        let claimed_rounds: Vec<&ShotFired> =
            rounds.iter().skip(next_round).take(claimed).collect();

        let mut touched: Vec<CellLevel> = seen.cell_of(shooter).into_iter().collect();
        for shot in &claimed_rounds {
            touched.extend(flight_of(shot));
        }
        log.append(RecordedAct::new(
            shooter,
            cause,
            ActDeed::Fired {
                target: declaration.target,
                mode:   declaration.mode,
                rounds: declaration.rounds,
            },
            witnesses(seen, &touched, shooter, cause),
        ));
        for shot in claimed_rounds {
            let flight = flight_of(shot);
            log.append(RecordedAct::new(
                shooter,
                cause,
                ActDeed::RoundResolved {
                    shot: Box::new(shot.clone()),
                },
                witnesses(seen, &flight, shooter, cause),
            ));
        }
        next_round = next_round.saturating_add(claimed);
    }
}

// Every cell the round passed through, muzzle to impact.
fn flight_of(shot: &ShotFired) -> Vec<CellLevel> {
    let impact = CellLevel::new(shot.impact_cell, shot.impact_level);
    cells_crossed(shot.muzzle, impact).cells().collect()
}

// Who saw the act, who could name the shooter, and who could name whoever it interrupted.
fn witnesses(
    seen: &ActObservation,
    touched: &[CellLevel],
    shooter: Entity,
    cause: ActProvenance,
) -> ActWitnesses {
    let witnesses = seen.of_effect(touched, shooter);
    match cause {
        ActProvenance::Reaction { interrupted } => witnesses.interrupting(seen.naming(interrupted)),
        _ => witnesses,
    }
}

//! The FIRE-family recorder — the declaration, its exact rounds, and the reaction
//! discriminator (GTW-727 C9 family 4, C5, C10).

use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::Entity};

use super::sources::{FireSources, ProvenanceSources};
use crate::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    shot_fired::ShotFired,
};

/// Record each fire act declared this tick, followed by EXACTLY the rounds that
/// declaration emitted.
///
/// ## Why the pairing is by COUNT, not by a leading run of matching shooters
///
/// A greedy "take the leading run of rounds whose shooter matches this declaration" rule
/// mis-attributes rounds in precisely the case this ticket exists to fix. The reaction
/// trigger documents a reactor evaluated twice in one pass, and the per-turn interrupt cap
/// `floor(cap_base + cap_per_reactions × Reactions)` is at least 2 for a Reactions-2
/// ganger, so ONE shooter can own TWO declarations in one tick; a greedy run would hand
/// the first declaration both volleys and leave the second with none. Instead
/// [`FireDeclaration`](crate::acts::FireDeclaration) carries the
/// [`RoundCount`](crate::acts::RoundCount) it actually emitted (written after the rounds
/// are emitted, C10), and this recorder consumes exactly that many from the volley-ordered
/// round stream.
///
/// ## Reaction provenance
///
/// The reaction trigger is ordered `.before(dispatch_fire)`, so an interrupt's
/// [`InterruptDeclared`](crate::reaction::InterruptDeclared) and the
/// [`FireDeclaration`](crate::acts::FireDeclaration) it produces land in the SAME tick. A
/// per-reactor QUEUE (not a single slot) covers the reactor that interrupts twice in one
/// pass, so a second interrupt cannot inherit the first one's interrupted actor.
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

    // Drain the whole tick's rounds UP FRONT, in volley order. The reader must advance
    // every run whether or not a declaration claims its rounds, so it never backs up.
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

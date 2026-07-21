//! [`record_acts`] — the ONE system that appends to the [`ActLog`] (GTW-727 C9 / C14).

use bevy::prelude::ResMut;

use super::{
    consequence::{record_consequence_messages, record_magazines, record_vitals},
    fire::record_fire,
    life::record_life,
    movement::record_movement,
    posture::record_posture,
    sources::{
        ConsequenceMessages, ConsequenceState, FireSources, LifeSources, MovementSources,
        PostureSources, ProvenanceSources, TurnSources,
    },
    turn::record_turn,
};
use crate::act_log::ActLog;

/// `Update` ([`SimSystems::Record`](crate::occupancy_sync::SimSystems::Record), gated on
/// [`BattleInProgress`](crate::battle::BattleInProgress)): append this tick's acts to the
/// [`ActLog`], in a FIXED order.
///
/// ## One writer, and the order is source code
///
/// This is the ONLY system that appends to the log. Its six per-family recorders are
/// invoked in a fixed call order — **turn → posture → movement → fire → consequence →
/// life** — and that call order IS the intra-tick ordering guarantee. Nothing about the
/// resulting sequence depends on the scheduler, so the log is reproducible run to run for
/// one seed; ordering ambiguity between independent forwarder systems (the pre-existing
/// hazard `bevy-traps.md` #3 warns about) cannot reach it.
///
/// The family order is not arbitrary. A turn's boundary precedes the acts inside it; a
/// posture change (turning to face a target) precedes the shot it enabled; a shot's rounds
/// precede the damage they did; the damage precedes the death it caused. A consumer that
/// replays the log in order therefore shows cause before effect by construction.
///
/// ## It touches ZERO combat-resolution code
///
/// Every source is an already-emitted output message or a read-only query. No dispatcher
/// gains a param, a [`ParamSet`](bevy::prelude::ParamSet) member, or a new query — which
/// is deliberate: `dispatch_fire` already holds a `ParamSet` of the shooter/turn queries
/// and a target query with `&mut Hp` / `&mut Wounds` / `&mut LifeState`, so a read-only
/// stat query bolted onto it as an independent param would be a startup access-conflict
/// panic, at every such site.
///
/// ## It never waits
///
/// The system appends and returns. There is no back-pressure, no capacity block, and no
/// presenter type named anywhere in the sim: an overflowing log drops its oldest entry and
/// counts it rather than stalling a writer. The sim's timing is identical with the log
/// wired and without it.
///
/// Param-only (`bevy-traps.md` #7): the log plus one [`SystemParam`](bevy::ecs::system::SystemParam)
/// bundle per family.
#[expect(
    clippy::too_many_arguments,
    reason = "the parameter list IS the recorder's source inventory: one SystemParam bundle \
              per act family, plus the log. Bundling them further would hide which family \
              reads what — the exact thing this system's fixed family order is documented \
              on — and Bevy's own arity ceiling is nowhere near being pressed (the sources \
              are already grouped into six bundles from ~20 underlying reads)"
)]
pub fn record_acts(
    mut log: ResMut<ActLog>,
    provenance: ProvenanceSources,
    mut turn: TurnSources,
    posture: PostureSources,
    mut movement: MovementSources,
    mut fire: FireSources,
    mut consequence_messages: ConsequenceMessages,
    consequence_state: ConsequenceState,
    life: LifeSources,
) {
    let log = &mut *log;
    record_turn(log, &mut turn, &provenance);
    record_posture(log, &posture, &provenance);
    record_movement(log, &mut movement, &provenance);
    record_fire(log, &mut fire, &provenance);
    record_consequence_messages(log, &mut consequence_messages, &provenance);
    record_vitals(log, &consequence_state, &provenance);
    record_magazines(log, &consequence_state, &provenance);
    record_life(log, &life, &provenance);
}

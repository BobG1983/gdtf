//! The GTW-328 per-shot [`ShotImpactResolved`] message.

use bevy::prelude::*;
use gdtf_battle_sim::HitReport;

/// A per-shot SHOT-IMPACT-RESOLVED signal — emitted (GTW-328) the instant each shot's
/// [`PendingImpact`](super::super::projectile::PendingImpact) is consumed in
/// [`animate_impact`](super::animate::animate_impact), i.e.
/// when the staggered bolt has flown and its impact lands.
///
/// This is the SHARED presenter-side per-shot impact moment the firing FX already keys off
/// (the floating-combat-text pops spawn here, GTW-327) — surfaced as a buffered
/// [`Message`](bevy::ecs::message::Message) so a downstream consumer can react at the SAME
/// staggered cadence. The combat-text LOG (`gdtf_app`) drains it to build a shot-outcome line PER
/// IMPACT (instead of dumping a whole volley's lines on the `ShotFired`-drain frame), and GTW-331
/// (death-despawn at impact) will reuse it. It carries exactly what a downstream needs to name +
/// classify the shot: the firing [`Entity`] and the sim's already-computed verdict.
///
/// Pure VIEW (ADR-0001): it is emitted from the presenter's own impact-resolution timing over data
/// the sim already produced (the [`HitReport`]); it adds NO sim plumbing and never writes the sim.
/// The [`shooter`](ShotImpactResolved::shooter) [`Entity`] is framework plumbing (the
/// no-bare-types carve-out), and [`report`](ShotImpactResolved::report) is the sim's own value type
/// — the consumer resolves the entity to a name + reuses the report through the shared classifier.
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-444): the [`HitReport`]'s rolled-injury effects
/// may include a `MovementCostMul` `f32` payload (not `Eq`). A buffered message is read in
/// order, never keyed in a hashed/ordered set.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ShotImpactResolved {
    /// The firing entity whose shot just impacted — the consumer resolves it to a display name.
    pub shooter: Entity,
    /// The sim's already-computed verdict for this shot ([`HitReport`]: damage / wound / DOWN /
    /// DEAD, or a no-effect miss). [`None`] means NO ganger-shot verdict exists — the grenade
    /// blast's detonation seed (GTW-546) — and the combat log renders NO outcome line for it
    /// (GTW-559: a blast is not a miss). Reused through the shared classifier — never
    /// recomputed.
    pub report:  Option<HitReport>,
}

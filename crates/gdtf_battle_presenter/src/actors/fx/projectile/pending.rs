//! The [`PendingImpact`] arrival seam FX-B reads to play the 3-frame impact —
//! including the GTW-546 blast seed.

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, DamageType, HitReport, Level};

use super::super::fct::ClassifiedPop;

/// A projectile has ARRIVED — the SEAM FX-B reads to play the 3-frame impact.
///
/// [`advance_projectiles`](super::advance::advance_projectiles) spawns one of these (a bare
/// entity carrying ONLY this
/// component) at the arrival point the instant a projectile despawns;
/// [`animate_impact`](super::super::impact::animate_impact) queries for them and steps the
/// [`damage`](PendingImpact::damage) type's 3 impact frames there (AND, GTW-327, spawns this
/// shot's floating-combat-text pops at the [`anchor`](PendingImpact::anchor)) before despawning
/// the impact entity. FX-A defines + spawns this so FX-B's `impact` module only fills the
/// animation
/// body (no `mod.rs` collision).
///
/// A NAMED grouping component: [`at`](PendingImpact::at) is the impact world point
/// (the [`Vec3`](bevy::math::Vec3) carve-out — framework geometry), `damage` the
/// shot's [`DamageType`] (so FX-B picks the matching impact strip), and — GTW-327 —
/// [`pops`](PendingImpact::pops) the shot's classified floating-combat-text pops +
/// [`anchor`](PendingImpact::anchor) the `(cell, level)` they sit on, so the numbers appear at
/// THIS shot's staggered impact. Not [`Copy`] (it owns the pop [`Vec`]); the
/// [`fields`](PendingImpact) are `pub(in crate::actors::fx)` so `animate_impact` reads + consumes them
/// while the type stays sealed to the FX layer.
#[derive(Component, Debug, Clone)]
pub struct PendingImpact {
    /// The world point the projectile arrived at — where the impact animation plays.
    pub(in crate::actors::fx) at:      Vec3,
    /// The shot's damage type — selects which 3-frame impact strip FX-B animates.
    pub(in crate::actors::fx) damage:  DamageType,
    /// This shot's classified floating-combat-text pops (GTW-327) — spawned by
    /// [`animate_impact`](super::super::impact::animate_impact) at the
    /// [`anchor`](PendingImpact::anchor)
    /// when the impact lands. Empty for a clean miss (no pops).
    pub(in crate::actors::fx) pops:    Vec<ClassifiedPop>,
    /// The `(cell, level)` this shot's pops anchor over (GTW-327) — the hit ganger's cell at
    /// the SHOT (not the impact cell), threaded through the staggered flight.
    pub(in crate::actors::fx) anchor:  (Cell, Level),
    /// The firing entity (GTW-328) — so [`animate_impact`](super::super::impact::animate_impact)
    /// names
    /// the shooter in the [`ShotImpactResolved`](super::super::impact::ShotImpactResolved)
    /// signal it
    /// emits when this impact resolves. The [`Entity`] is framework plumbing (the no-bare-types
    /// carve-out).
    pub(in crate::actors::fx) shooter: Entity,
    /// This shot's hit report (GTW-328) — the sim's verdict, carried into the
    /// [`ShotImpactResolved`](super::super::impact::ShotImpactResolved) signal so the combat log
    /// classifies the shot outcome at THIS shot's staggered impact. [`None`] carries no
    /// verdict (the blast seed) — the log renders no outcome line for it (GTW-559).
    pub(in crate::actors::fx) report:  Option<HitReport>,
}

impl PendingImpact {
    /// Seed a BLAST impact at a lobbed grenade's landing world point `at` carrying `damage`
    /// (GTW-546) — the seam [`animate_impact`](super::super::impact::animate_impact) reads to
    /// play the
    /// grenade's damage-type 3-frame expanding-shockwave strip at the detonation point.
    ///
    /// Unlike a shot's arrival seed (which carries this shot's classified pops + the shooter +
    /// verdict for the GTW-328 [`ShotImpactResolved`](super::super::impact::ShotImpactResolved)
    /// line), a
    /// blast is a MULTI-ganger fan with no single per-shot verdict: its numbers / downs ride the
    /// per-ganger wound / injury / bleed FCT signals the sim's blast fold already drives. So this
    /// seed carries an EMPTY pop list, a [`None`] report, and a
    /// [`PLACEHOLDER`](bevy::ecs::entity::Entity::PLACEHOLDER) shooter —
    /// [`animate_impact`](super::super::impact::animate_impact)'s
    /// emitted `ShotImpactResolved` then carries no verdict, and the combat log renders NO
    /// outcome line for it (GTW-559 — no phantom miss). Kept
    /// `pub(in crate::actors::fx)` so only the sibling blast reader
    /// ([`read_throw_resolved`](super::super::blast::read_throw_resolved)) constructs it (the FX
    /// layer
    /// owns the seam).
    #[must_use]
    pub(in crate::actors::fx) const fn for_blast(at: Vec3, damage: DamageType) -> Self {
        Self {
            at,
            damage,
            pops: Vec::new(),
            anchor: (Cell::new(0, 0), Level::new(0)),
            shooter: Entity::PLACEHOLDER,
            report: None,
        }
    }
}

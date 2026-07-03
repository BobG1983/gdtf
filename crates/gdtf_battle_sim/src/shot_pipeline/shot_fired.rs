//! The [`ShotFired`] signal — the per-round fire-trajectory message the presenter
//! reads to draw the GTW-290 muzzle / tracer / impact FX.
//!
//! The shot's geometry is ALREADY computed by the coarse pipeline
//! ([`ShotOutcome`](crate::resolve_coarse::ShotOutcome) in
//! [`resolve_coarse`](crate::resolve_coarse::resolve_coarse)) and folded into the
//! frozen volley [`fire`](crate::fire::fire) returns; this message simply EXPOSES that
//! already-resolved geometry so it is not dropped. It carries NO new fire-result logic
//! and recomputes nothing — every field is copied straight off the round's
//! [`ShotOutcome`].
//!
//! A buffered Bevy **message** (`#[derive(Message)]`), mirroring
//! [`crate::bleed::Bleeding`] / [`crate::occupancy_sync::CoverDestroyed`] /
//! [`crate::armor_wear::ArmorBroken`] — NOT the observer `Event` API
//! (`bevy-traps.md` #4), so it is written with [`bevy::prelude::MessageWriter`] and read
//! with [`bevy::prelude::MessageReader`]. One [`ShotFired`] is emitted per ROUND resolved
//! in [`dispatch_fire`](crate::acts::dispatch_fire) (a burst / full-auto volley fires
//! multiple rounds → multiple [`ShotFired`], so the presenter draws a tracer per round).
//!
//! The sim stays RENDER-FREE: every position rides in **sim units** — a cubic-voxel
//! [`SimPos`] muzzle, a unit-[`Vec3`](bevy::math::Vec3) [`ShotDir`] trajectory, and a
//! discrete `(`[`Cell`]`, `[`Level`]`)` impact — never a pixel, never a sprite, never a
//! colour. The presenter maps those sim units to screen space; the sim never reads the
//! presenter (the one-way `input -> presenter -> sim` edge, ADR-0001).

use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level, SimPos},
    resolve_and_apply::HitReport,
    resolve_coarse::{ShotKind, ShotOutcome},
    sample_cone::ShotDir,
    weapon::DamageType,
};

/// One **round was fired** — the per-round trajectory geometry the presenter draws the
/// GTW-290 muzzle / tracer / impact FX from.
///
/// Emitted once per round resolved in [`dispatch_fire`](crate::acts::dispatch_fire),
/// AFTER the volley resolves (so the geometry is final), sourced field-by-field from the
/// round's already-computed [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) — it adds
/// no fire-result logic and recomputes nothing.
///
/// Every position is a **sim unit**, never a pixel: [`muzzle`](ShotFired::muzzle) is the
/// 3D fire origin ([`SimPos`]), [`trajectory`](ShotFired::trajectory) is the round's unit
/// direction ([`ShotDir`]), and [`impact_cell`](ShotFired::impact_cell) /
/// [`impact_level`](ShotFired::impact_level) are the discrete impact `(cell, level)`. The
/// [`kind`](ShotFired::kind) is the [`ShotKind`] the shot struck (ganger / cover / slab /
/// ground / miss) — a clean miss still carries muzzle + trajectory + the cell the round
/// left through, so the presenter draws a tracer terminating at the impact cell.
///
/// The [`report`](ShotFired::report) (GTW-302) carries the round's already-computed
/// [`HitReport`] — the damage / wound / severity / armor verdict the floating-combat-text
/// presenter draws from — so the ONE message drives BOTH the firing FX and the FCT. It is
/// PURE EXPOSURE of the parallel [`Volley::reports`](crate::fire::Volley::reports) entry
/// (the report `dispatch_fire` would otherwise drop), NOT a recompute: no fire-result,
/// RNG, or severity logic runs to build it.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`). The
/// [`shooter`](ShotFired::shooter) is a Bevy [`Entity`] handle — framework plumbing, the
/// only bare type the no-bare-types rule permits in a payload. Derives [`PartialEq`] (NOT
/// [`Eq`]: the [`SimPos`] / [`ShotDir`] hold `f32`, so equality is bit-wise, the
/// seeded-replay property).
///
/// GTW-438: [`ShotFired`] is [`Clone`] but NOT `Copy` — its [`report`](ShotFired::report)
/// carries a [`HitReport`], which since GTW-438 holds the rolled
/// [`RolledInjury`](crate::injuries::RolledInjury) (an owned `Vec` of effects + texts).
/// The per-round emission clones the report into the message (the FCT presenter reads the
/// verdict; the injury rides the separate
/// [`InjuryInflicted`](crate::acts::InjuryInflicted)).
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ShotFired {
    /// The firing entity (the armed shooter the round left).
    pub shooter:      Entity,
    /// The 3D muzzle point the round was fired from, in sim units (a cubic-voxel
    /// [`SimPos`]) — the tracer's origin + the muzzle-flash position.
    pub muzzle:       SimPos,
    /// The round's sampled trajectory — the ONE 3D unit direction the cone draw produced
    /// ([`ShotDir`]). A sim-space unit vector, never a pixel.
    pub trajectory:   ShotDir,
    /// The `(cell, *)` ground cell the round impacted — the tracer's terminus + the
    /// impact-flash position.
    pub impact_cell:  Cell,
    /// The storey [`Level`] of the impact (paired with [`impact_cell`](ShotFired::impact_cell)).
    pub impact_level: Level,
    /// What the round struck ([`ShotKind`]: ganger / cover / slab / ground / miss) — the
    /// presenter may branch the impact mark on it; a miss still draws muzzle + tracer.
    pub kind:         ShotKind,
    /// The firing weapon's [`DamageType`] (GTW-306) — the weapon-side wheel node the round
    /// carries (`docs/combat/matchup.md` §"The 7 types"). Known at FIRE time (sourced from
    /// the shooter's [`DamageType`] component in
    /// [`dispatch_fire`](crate::acts::dispatch_fire)), so it rides on HIT and MISS alike;
    /// the presenter selects the per-damage-type projectile graphic from it (a domain enum,
    /// never a sprite / pixel — the sim stays render-free).
    pub damage:       DamageType,
    /// The round's already-computed damage / wound / severity / armor verdict (GTW-302) —
    /// the [`HitReport`] the floating-combat-text presenter reads (its per-kind
    /// [`HitVerdict`](crate::resolve_and_apply::HitVerdict): the ganger verdict's HP
    /// damage / [`Severity`](crate::severity::Severity) tier / struck
    /// [`BodyPart`](crate::armor::BodyPart) / [`LifeState`](crate::ganger::LifeState)
    /// after / [`ArmorWearOutcome`](crate::armor_wear::ArmorWearOutcome), or the
    /// structural destruction / accrual verdicts). It is PURE EXPOSURE of the parallel
    /// [`Volley::reports`](crate::fire::Volley::reports) entry — the same `reports[i]` the
    /// volley already produced — never a recompute (no fire-result / RNG / severity logic).
    ///
    /// `None` ONLY when the round has no parallel report to carry: that happens when
    /// [`from_outcome`](ShotFired::from_outcome) builds the message WITHOUT a report (e.g.
    /// the GTW-290 geometry-only callers). In `dispatch_fire`'s zip, every fired round has
    /// its `reports[i]`, so the report is always `Some` for a real volley round (a clean
    /// MISS still carries `Some` — a [`HitReport`] whose [`kind`](HitReport::kind) is
    /// [`ShotKind::Miss`](crate::resolve_coarse::ShotKind::Miss) and whose verdict is
    /// no-effect). A non-ganger / corpse-skip round carries `Some` with a no-effect
    /// verdict.
    pub report:       Option<HitReport>,
}

impl ShotFired {
    /// Build a **geometry-only** [`ShotFired`] for `shooter` firing `damage` from the
    /// round's already-computed [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) — the
    /// [`report`](ShotFired::report) is `None`.
    ///
    /// Copies the geometry straight off the outcome — muzzle / trajectory / impact
    /// `(cell, level)` / kind — and pairs it with the weapon's [`DamageType`] (GTW-306),
    /// which is NOT part of the coarse outcome (it is weapon-side, read from the shooter's
    /// [`DamageType`] component at fire time in
    /// [`dispatch_fire`](crate::acts::dispatch_fire)). So the message EXPOSES the resolved
    /// trajectory plus the damage type WITHOUT recomputing or changing any fire-result
    /// logic — pure exposure.
    ///
    /// Use [`from_round`](ShotFired::from_round) when the parallel
    /// [`Volley::reports`](crate::fire::Volley::reports) entry is available (the
    /// `dispatch_fire` path), so the FCT presenter (GTW-302) gets the damage / wound /
    /// severity / armor verdict; this constructor is the geometry-only fallback.
    #[must_use]
    pub const fn from_outcome(shooter: Entity, damage: DamageType, outcome: &ShotOutcome) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
            damage,
            report: None,
        }
    }

    /// Build a [`ShotFired`] for `shooter` firing `damage` from the round's already-computed
    /// [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) AND its parallel
    /// [`HitReport`] (GTW-302) — the report the floating-combat-text presenter draws from.
    ///
    /// The geometry is copied exactly as [`from_outcome`](ShotFired::from_outcome) does; the
    /// `report` is the round's matching [`Volley::reports`](crate::fire::Volley::reports)
    /// entry, carried verbatim onto [`report`](ShotFired::report). PURE EXPOSURE of the
    /// already-computed report — no fire-result / RNG / severity logic runs here. In
    /// `dispatch_fire`'s zip of `shots[i]` with `reports[i]`, `report` is always `Some`; a
    /// clean MISS rides `Some` with a [`ShotKind::Miss`](crate::resolve_coarse::ShotKind::Miss)
    /// report (no `applied`).
    #[must_use]
    pub const fn from_round(
        shooter: Entity,
        damage: DamageType,
        outcome: &ShotOutcome,
        report: HitReport,
    ) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
            damage,
            report: Some(report),
        }
    }
}

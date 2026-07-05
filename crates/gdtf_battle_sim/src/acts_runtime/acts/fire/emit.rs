//! The per-round exhaustive verdict→message bridge — expose the volley the fire
//! already produced onto the boundary signals (no recompute, no extra RNG draw).

use bevy::prelude::Entity;

use super::signals::FireSignals;
use crate::{
    acts::injury::InjuryInflicted,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    shot_fired::ShotFired,
    weapon::DamageType,
};

/// Emit the per-ROUND output signals for a resolved `volley` — the GTW-290/302 [`ShotFired`]
/// FX/FCT plus the ONE exhaustive per-kind boundary bridge ([`emit_report_signals`]).
///
/// One signal set per fired round, zipping the parallel
/// [`Volley::shots`](crate::fire::Volley::shots) geometry with the
/// [`Volley::reports`](crate::fire::Volley::reports) verdicts (`shots[i]`/`reports[i]` are
/// the same round); every SPLASH report (GTW-541 — the blast / cone / line's other
/// occupants, EMPTY for a Single volley) rides the SAME bridge, so the primary and splash
/// loops cannot drift. Every emission is PURE EXPOSURE of what the volley already computed
/// — no recompute, no fire-result change, no extra RNG draw (the injury roll happened
/// in-fold, frozen on the verdict). Extracted from [`dispatch_fire`](super::dispatch::dispatch_fire) so that system stays
/// under clippy's line-count gate; takes the writer bundle by `&mut` (the
/// [`MessageWriter`](bevy::prelude::MessageWriter)s).
pub(super) fn emit_round_signals(
    shooter: Entity,
    damage: DamageType,
    volley: &crate::fire::Volley,
    signals: &mut FireSignals,
) {
    for (outcome, report) in volley.shots.iter().zip(volley.reports.iter()) {
        // (5a) The ONE exhaustive per-kind bridge — injury / DOT / on-death for a ganger
        //      verdict, CoverDestroyed(+cover on-death) / SlabDestroyed / GroundAccrued
        //      for the structural verdicts (GTW-573 C4).
        emit_report_signals(report, signals);
        // (5b) The HitReport is non-`Copy` (it carries the rolled injury); clone it into
        //      the per-round ShotFired (the FCT presenter reads the damage/wound/severity
        //      verdict — the injury rides the separate InjuryInflicted).
        signals.shots.write(ShotFired::from_round(
            shooter,
            damage,
            outcome,
            report.clone(),
        ));
    }
    // (5c) GTW-541 (`AoE` CORE of GTW-41): the SPLASH bridge. A non-Single round's
    //      template covers OTHER occupants; each was applied to the world through the
    //      SAME resolve_and_apply path in-fold (HP / wounds already mutated). Each splash
    //      report rides the SAME exhaustive bridge as the primary loop (GTW-573 C4 — one
    //      bridge, not a repeated per-kind probe set), so a splash victim's injury / DOT /
    //      on-death lands identically. `volley.splash` is EMPTY for a Single volley, so
    //      this loop is a no-op on the unchanged single-target path (the identity
    //      property); a splash verdict is always ganger-or-no-effect, so the structural
    //      arms are dead here by construction. No RNG draw / no recompute.
    for round_splash in &volley.splash {
        for report in round_splash {
            emit_report_signals(report, signals);
        }
    }
}

/// Bridge ONE frozen [`HitReport`](crate::resolve_and_apply::HitReport) into its per-kind
/// boundary messages — the ONE **exhaustive** verdict match of the fire boundary
/// (GTW-573 C4): a new struck kind is a COMPILE ERROR here until it is bridged.
///
/// The per-kind message TYPES and their focused drains are unchanged (the Bevy idiom —
/// this changes emission, not consumers):
///
/// - a **ganger** verdict emits the GTW-438 [`InjuryInflicted`] (a rolled named injury),
///   the GTW-544 [`DotApplied`](crate::acts_runtime::dot::DotApplied) (a penetrating DOT
///   hit), and the GTW-547 [`OnDeathOccurred`](crate::on_death::OnDeathOccurred) (the
///   frozen `life_after == Dead` kill verdict, at the ganger's OWN cell read off its
///   [`Position`](crate::ganger::Position) — a splash report carries no cell); each is addressed to the verdict's
///   own struck target;
/// - a **cover** verdict whose HP depleted to zero emits [`CoverDestroyed`] PLUS the
///   GTW-547 cover terminal-death signal (keyed by cell — cover is not an entity);
/// - a **slab** verdict whose HP depleted to zero emits [`SlabDestroyed`];
/// - a **ground** verdict emits [`GroundAccrued`] (damaged-never-destroyed — accrual,
///   not destruction);
/// - a **no-effect** verdict (miss / corpse-skip / defensive fold) emits nothing.
///
/// No RNG draw, no recompute — pure exposure of the fold's frozen verdict.
pub(super) fn emit_report_signals(
    report: &crate::resolve_and_apply::HitReport,
    signals: &mut FireSignals,
) {
    use crate::resolve_and_apply::HitVerdict;
    match &report.verdict {
        HitVerdict::Ganger(verdict) => {
            // GTW-438: the injury bridge — the in-fold `roll_injury` already took its ONE
            // severity-gated InjuryRng draw; `apply_injury` drains the message (folds the
            // GainedInjury into the target's InflictedInjuries + syncs the bleed); the
            // presenter (GTW-439) reads it for the FCT/log flash.
            if let Some(rolled) = &verdict.injury {
                signals
                    .injuries
                    .write(InjuryInflicted::from_rolled(verdict.target, rolled.clone()));
            }
            // GTW-544: the DOT bridge — `apply_dot` attaches (or REFRESHES —
            // refresh-not-stack) the frozen Dot on the struck ganger.
            if let Some(dot) = verdict.dot_applied {
                signals
                    .dots
                    .write(crate::acts_runtime::dot::DotApplied::new(
                        verdict.target,
                        dot,
                    ));
            }
            // GTW-572: the armor-broken bridge — a §6 wear that crossed the struck worn
            // piece from protecting to broken emits the buffered ArmorBroken fact (the
            // presenter spark/tag + the combat log's armor-broken line drain it). Damaged /
            // Unaffected outcomes emit nothing (the "emit only on the crossing" rule).
            if let crate::armor_wear::ArmorWearOutcome::Broke(broken) = verdict.applied.wear {
                signals.armor_breaks.write(broken);
            }
            // GTW-547: the on-death bridge — a KILL verdict fans the dead ganger's
            // authored on-death effect via `resolve_on_death`. A round that missed,
            // wounded-but-did-not-kill, or downed (Hp → 0) emits nothing.
            if verdict.applied.life_after == crate::ganger::LifeState::Dead
                && let Ok(position) = signals.ganger_positions.get(verdict.target)
            {
                signals.deaths.write(crate::on_death::OnDeathOccurred::new(
                    verdict.target,
                    **position,
                ));
            }
        }
        HitVerdict::Cover(cover) => {
            // GTW-364: the cover fire→deplete→message bridge — a round that depleted
            // cover to zero carries the destroyed (cell, level).
            if let Some(at) = cover.destroyed {
                signals.cover_destroyed.write(CoverDestroyed::new(at));
                // GTW-547: a destroyed piece of cover ALSO emits a terminal-death signal
                // (keyed by its cell — cover is not an entity, so Entity::PLACEHOLDER) so
                // `resolve_on_death` fans the cover tile's authored on-death effect (a
                // fuel barrel leaving a field).
                signals
                    .deaths
                    .write(crate::on_death::OnDeathOccurred::cover(at));
            }
        }
        HitVerdict::Slab(slab) => {
            // GTW-365: the slab mirror — a round that depleted a slab to zero carries the
            // destroyed (cell, level).
            if let Some(at) = slab.destroyed {
                signals.slab_destroyed.write(SlabDestroyed::new(at));
            }
        }
        HitVerdict::Ground(accrual) => {
            // GTW-366: the ground-accrual bridge (the ground is damaged-never-destroyed —
            // accrual, not destruction).
            signals
                .ground_accrued
                .write(GroundAccrued::new(accrual.cell, accrual.amount));
        }
        // A no-effect fold (miss / corpse-skip / defensive) crossed no boundary.
        HitVerdict::NoEffect => {}
    }
}

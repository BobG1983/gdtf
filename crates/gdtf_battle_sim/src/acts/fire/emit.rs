//! Turn volley reports into shot, injury, terrain, and death messages.

use bevy::prelude::Entity;

use super::signals::FireSignals;
use crate::{
    acts::injury::InjuryInflicted,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    shot_fired::ShotFired,
    weapon::DamageType,
};

/// Emit per-round shot messages and side effects from a resolved volley.
pub(super) fn emit_round_signals(
    shooter: Entity,
    damage: DamageType,
    volley: &crate::fire::Volley,
    signals: &mut FireSignals,
) {
    for (outcome, report) in volley.shots.iter().zip(volley.reports.iter()) {
        emit_report_signals(report, signals);
        signals.shots.write(ShotFired::from_round(
            shooter,
            damage,
            outcome,
            report.clone(),
        ));
    }
    for round_splash in &volley.splash {
        for report in round_splash {
            emit_report_signals(report, signals);
        }
    }
}

/// Emit injury, DOT, armor break, death, or structure destruction from one hit report.
pub(super) fn emit_report_signals(
    report: &crate::resolve_and_apply::HitReport,
    signals: &mut FireSignals,
) {
    use crate::resolve_and_apply::HitVerdict;
    match &report.verdict {
        HitVerdict::Ganger(verdict) => {
            if let Some(rolled) = &verdict.injury {
                signals
                    .injuries
                    .write(InjuryInflicted::from_rolled(verdict.target, rolled.clone()));
            }
            if let Some(dot) = verdict.dot_applied {
                signals
                    .dots
                    .write(crate::effects::dot::DotApplied::new(verdict.target, dot));
            }
            if let crate::armor_wear::ArmorWearOutcome::Broke(broken) = verdict.applied.wear {
                signals.armor_breaks.write(broken);
            }
            if verdict.applied.life_after == crate::ganger::LifeState::Dead
                && let Ok(position) = signals.ganger_positions.get(verdict.target)
            {
                signals
                    .deaths
                    .write(crate::effects::on_death::OnDeathOccurred::new(
                        verdict.target,
                        **position,
                    ));
            }
        }
        HitVerdict::Cover(cover) => {
            if let Some(at) = cover.destroyed {
                signals.cover_destroyed.write(CoverDestroyed::new(at));
                signals
                    .deaths
                    .write(crate::effects::on_death::OnDeathOccurred::cover(at));
            }
        }
        HitVerdict::Slab(slab) => {
            if let Some(at) = slab.destroyed {
                signals.slab_destroyed.write(SlabDestroyed::new(at));
            }
        }
        HitVerdict::Ground(accrual) => {
            signals
                .ground_accrued
                .write(GroundAccrued::new(accrual.cell, accrual.amount));
        }
        HitVerdict::NoEffect => {}
    }
}

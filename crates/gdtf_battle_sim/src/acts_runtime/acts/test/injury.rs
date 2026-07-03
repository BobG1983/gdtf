//! GTW-550 — the SAME-TICK seam proof for the injury message-drain path: an
//! [`InjuryInflicted`](crate::acts::InjuryInflicted) drained by
//! [`apply_injury`](crate::acts::apply_injury) folds through the palette trait
//! SYNCHRONOUSLY (never a deferred command), so the gain trips
//! `Changed<InflictedInjuries>` and the GTW-436 projector lands the stat delta on the
//! derived stats within the SAME `Update` pass — the load-bearing ordering the GTW-550
//! refinement pins (a `Commands`-deferred gain would land a tick late and FAIL this).
//!
//! Mirrors the live wiring: `apply_injury` runs in the `SimSystems::Simulate` band and
//! `rederive_stats_on_injury_change` is ordered `.after(SimSystems::Simulate)` in the
//! same `Update` schedule — reproduced here as a `.chain()` of the two real systems.

use bevy::{
    MinimalPlugins,
    prelude::{App, Entity, IntoScheduleConfigs, Update, World},
};

use crate::{
    acts::{InjuryInflicted, apply_injury},
    armor::BodyPart,
    ganger::{
        Aim, Cool, GangerAttributes, Grit, Luck, Reflexes, Shooting, Speed, Strength, Toughness,
        derive_stats_with_injuries, rederive_stats_on_injury_change,
    },
    injuries::{
        InflictedInjuries, InjuryEffect, InjuryName, InspectText, LogText, PopupText, RolledInjury,
        StatDelta, StatTarget,
    },
    severity::Severity,
    tuning::GangerStatTuning,
};

/// An arbitrary (NOT shipped) set of distinct attribute magnitudes (mirrors the
/// GTW-436 projector suite's fixture).
fn sample_attributes() -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(4.0),
        aim:       Aim::new(6.0),
        strength:  Strength::new(5.0),
        toughness: Toughness::new(11.0),
        reflexes:  Reflexes::new(3.0),
        cool:      Cool::new(7.0),
        grit:      Grit::new(20.0),
        luck:      Luck::new(2.0),
    }
}

/// Spawn one ganger carrying its eight attribute components + its derived components
/// (seeded by the injury-aware projector over an EMPTY ledger) + the empty ledger —
/// exactly the shape the live setup produces. A unit-test world-mutation carve-out
/// (bevy-traps #7a).
fn spawn_statted_ganger(world: &mut World, tuning: &GangerStatTuning) -> Entity {
    let a = sample_attributes();
    let ledger = InflictedInjuries::default();
    let d = derive_stats_with_injuries(&a, tuning, &ledger);
    world
        .spawn((
            a.speed,
            a.aim,
            a.strength,
            a.toughness,
            a.reflexes,
            a.cool,
            a.grit,
            a.luck,
        ))
        .insert((d.shooting, d.fight, d.reactions, d.morale))
        .insert((
            d.tu,
            d.tu_max,
            d.hp,
            d.hp_max,
            d.wounds,
            d.wounds_max,
            d.bottle,
        ))
        .insert(ledger)
        .id()
}

/// Build the [`InjuryInflicted`] the fire-act bridge would emit for one
/// `Modify(Shooting, penalty)` injury (through the REAL [`RolledInjury`] →
/// [`InjuryInflicted::from_rolled`] constructor, the GTW-438 path).
fn shooting_penalty_message(target: Entity, penalty: i8) -> InjuryInflicted {
    InjuryInflicted::from_rolled(
        target,
        RolledInjury::new(
            InjuryName::new("seam_probe".to_owned()),
            BodyPart::Torso,
            Severity::Minor,
            vec![InjuryEffect::Modify {
                stat:   StatTarget::Shooting,
                amount: StatDelta::new(penalty),
            }],
            PopupText::new("SEAM".to_owned()),
            LogText::new("probes the seam".to_owned()),
            InspectText::new("seam probe".to_owned()),
        ),
    )
}

/// THE SAME-TICK PROOF (GTW-550 C2): one buffered `InjuryInflicted`, ONE
/// `app.update()`, and the derived `Shooting` has ALREADY dropped by exactly the
/// penalty — the drain-path gain delegates through the palette trait synchronously,
/// and the `Changed<InflictedInjuries>` projector (ordered after the Simulate band,
/// same `Update`) lands the delta within that pass. Pin-discriminating: a
/// `Commands`-deferred gain would leave `Shooting` at its baseline after this single
/// tick (the projection would slip to the NEXT update), failing the equality.
#[test]
fn injury_message_lands_stat_delta_the_same_tick() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<InjuryInflicted>()
        // The live ordering, reproduced: apply_injury (Simulate band) strictly before
        // the projector (`.after(SimSystems::Simulate)`), in ONE Update schedule.
        .add_systems(
            Update,
            (apply_injury, rederive_stats_on_injury_change).chain(),
        );
    let tuning = GangerStatTuning::default();
    app.insert_resource(tuning.clone());
    let entity = spawn_statted_ganger(app.world_mut(), &tuning);
    // Settle the spawn tick (the fresh ledger's Added-implies-Changed projection).
    app.update();
    let Some(baseline) = app.world().get::<Shooting>(entity).map(|s| **s) else {
        unreachable!("the statted ganger must carry a derived Shooting");
    };

    // Buffer the injury message, then run EXACTLY ONE update.
    let penalty: i8 = -3;
    app.world_mut()
        .write_message(shooting_penalty_message(entity, penalty));
    app.update();

    // SAME TICK: the ledger gained the injury AND the projector landed the delta.
    let gained = app
        .world()
        .get::<InflictedInjuries>(entity)
        .map_or(0, |l| l.gained().len());
    assert_eq!(gained, 1, "apply_injury drained the message this tick");
    let after = app.world().get::<Shooting>(entity).map(|s| **s);
    assert_eq!(
        after,
        Some(baseline + f32::from(penalty)),
        "the Modify(Shooting) delta must land on the derived stat within the SAME \
         update as the message drain (a deferred gain would leave the baseline)",
    );
}

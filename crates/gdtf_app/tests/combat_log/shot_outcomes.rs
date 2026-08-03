use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::CombatLogLine;
use gdtf_battle_presenter::ShotImpactResolved;
use gdtf_battle_sim::{
    armor::BodyPart,
    armor_wear::ArmorWearOutcome,
    matchup::Matchup,
    prelude::{Cell, LifeState},
    resolve_and_apply::{AppliedDamage, GangerVerdict, HitReport, HitVerdict},
    resolve_coarse::ShotKind,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    severity::Severity,
    shot_fired::ShotFired,
};

use super::harness::*;

fn connecting_report(struck: Entity, hp: i32) -> HitReport {
    HitReport {
        kind:    ShotKind::Ganger(struck),
        verdict: HitVerdict::Ganger(Box::new(GangerVerdict {
            target:      struck,
            part:        BodyPart::Torso,
            applied:     AppliedDamage {
                matchup:    Matchup::Neutral,
                hit:        HitResult {
                    penetrating: PenetratingDamage::new(0),
                    hp_damage:   HpDamage::new(hp),
                    wear:        IntegrityWear::new(0),
                },
                severity:   Severity::None,
                life_after: LifeState::Alive,
                wear:       ArmorWearOutcome::Unaffected,
            },
            injury:      None,
            dot_applied: None,
        })),
    }
}

fn outcome_line_count(app: &mut App, hp_text: &str) -> usize {
    line_texts::<CombatLogLine>(app)
        .into_iter()
        .filter(|t| t == hp_text)
        .count()
}


#[test]
fn a_shot_impact_resolved_appends_the_outcome_line() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    app.world_mut().write_message(ShotImpactResolved {
        shooter,
        report: Some(connecting_report(struck, 7)),
    });
    app.update();

    assert_eq!(
        outcome_line_count(&mut app, "-7"),
        1,
        "a ShotImpactResolved carrying a 7-HP connecting hit must append one \"-7\" outcome line, \
         got {:?}",
        line_texts::<CombatLogLine>(&mut app),
    );
}

#[test]
fn a_shot_fired_no_longer_appends_an_outcome_line() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();

    app.world_mut().write_message(ShotFired {
        shooter,
        muzzle: gdtf_battle_sim::metric::SimPos::new(1.0, 1.0, 0.0),
        trajectory: gdtf_battle_sim::sample_cone::ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: Cell::new(2, 1),
        impact_level: gdtf_battle_sim::metric::Level::new(0),
        kind: ShotKind::Ganger(struck),
        damage: gdtf_battle_sim::weapon::DamageType::Kinetic,
        report: Some(connecting_report(struck, 7)),
    });
    app.update();
    app.update();

    assert_eq!(
        outcome_line_count(&mut app, "-7"),
        0,
        "a ShotFired must NOT append a shot-outcome line — the log keys outcomes off \
         ShotImpactResolved now, got {:?}",
        line_texts::<CombatLogLine>(&mut app),
    );
}

#[test]
fn outcome_lines_appear_staggered_one_per_impact_not_all_at_once() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();

    app.update();
    assert_eq!(
        outcome_line_count(&mut app, "-3"),
        0,
        "at the drain frame (no impact resolved yet) there must be ZERO shot-outcome lines",
    );

    app.world_mut().write_message(ShotImpactResolved {
        shooter,
        report: Some(connecting_report(struck, 3)),
    });
    app.update();
    let after_first = outcome_line_count(&mut app, "-3");
    assert_eq!(
        after_first,
        1,
        "after the first shot's ShotImpactResolved exactly one \"-3\" outcome line must exist, \
         got {:?}",
        line_texts::<CombatLogLine>(&mut app),
    );

    app.world_mut().write_message(ShotImpactResolved {
        shooter,
        report: Some(connecting_report(struck, 3)),
    });
    app.update();
    let after_second = outcome_line_count(&mut app, "-3");
    assert!(
        after_second > after_first,
        "after the second shot's staggered impact MORE \"-3\" outcome lines must exist \
         ({after_second} must exceed {after_first}) — the lines appeared one per impact, not at once",
    );
}

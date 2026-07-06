//! GTW-328 per-shot outcome lines: impact-resolved appends, fired no longer appends, staggered one-per-impact.

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

/// A connecting ganger-hit `HitReport` dealing `hp` HP to `struck`'s torso — the verdict a
/// `ShotImpactResolved` (or `ShotFired`) carries for a shot that landed (classifies to a
/// damage line). A no-effect report would read as a miss, and a `None` report yields no line
/// at all (GTW-559); this proves the connecting path. (Not `const`: the GTW-573 ganger
/// verdict is boxed, and `Box::new` is not const.)
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

/// The number of shot-OUTCOME log lines currently visible — the lines whose text is a
/// `classify_report` pop (here the `"-N"` HP-loss line a connecting hit yields). Distinguishes the
/// staggered outcome lines from any event-driven fire-declaration / movement / turn lines.
fn outcome_line_count(app: &mut App, hp_text: &str) -> usize {
    line_texts::<CombatLogLine>(app)
        .into_iter()
        .filter(|t| t == hp_text)
        .count()
}

// ---------------------------------------------------------------------------------
// GTW-328 slice A — shot-outcome lines key off the presenter's per-shot
// `ShotImpactResolved` signal (staggered per impact), NOT the fire-frame `ShotFired` drain.
// ---------------------------------------------------------------------------------

/// A `ShotImpactResolved` (the presenter's per-shot impact signal) makes the log gain ONE
/// shot-outcome line — the `classify_report` `"-N"` HP-loss line — with the shooter resolved to its
/// `GangerName`. This is the rewired drain (the log used to drain `ShotFired`).
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

/// The drain SWITCHED (GTW-328 clause 2): writing a `ShotFired` directly produces NO shot-outcome
/// line — the log no longer drains `ShotFired` for outcomes (it drains `ShotImpactResolved`). Only
/// the per-shot impact signal yields an outcome line.
#[test]
fn a_shot_fired_no_longer_appends_an_outcome_line() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();

    // Write a ShotFired carrying a connecting report — the OLD code would have logged its outcome
    // here, on the drain frame. The buffer is registered by the sim plugins in a live battle.
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

/// A MULTI-ROUND volley's shot-outcome lines appear STAGGERED — ONE PER `ShotImpactResolved`, as
/// each shot's impact resolves — NOT all at once. At the drain frame (no signal yet) the log has
/// ZERO outcome lines; each subsequent staggered `ShotImpactResolved` grows the outcome-line count
/// monotonically. (The presenter's `fx_draw` test proves the SIGNALS themselves fire staggered;
/// this proves the log turns each into exactly one outcome line, in cadence.)
#[test]
fn outcome_lines_appear_staggered_one_per_impact_not_all_at_once() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();

    // The drain frame: no ShotImpactResolved written -> zero outcome lines (the bug was a whole
    // volley's lines dumping here when the log drained ShotFired).
    app.update();
    assert_eq!(
        outcome_line_count(&mut app, "-3"),
        0,
        "at the drain frame (no impact resolved yet) there must be ZERO shot-outcome lines",
    );

    // First shot's impact resolves -> exactly one outcome line.
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

    // Second shot's impact resolves later -> the outcome-line count STRICTLY GROWS (shot-by-shot).
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

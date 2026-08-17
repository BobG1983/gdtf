use std::time::Duration;

use gdtf_battle_presenter::{FctValence, PlaybackCursor, valence_color};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    acts::RoundCount,
    armor::BodyPart,
    prelude::{BattleInProgress, Cell, CellLevel, Level, LifeState, Position, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    weapon::{DamageType, ModeKind},
};

use super::{harness::*, probes::*};

fn connecting_shot(app: &mut bevy::app::App, struck: bevy::ecs::entity::Entity) -> ShotFired {
    let report = ganger_hit_report(
        struck,
        BodyPart::Torso,
        9,
        6,
        Severity::Critical,
        LifeState::Dead,
    );
    ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 4.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  Cell::new(6, 4),
        impact_level: Level::new(0),
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    }
}

#[test]
fn a_raw_unplayed_shot_fired_pops_nothing() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(6, 4);
    let level = Level::new(0);
    let struck = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    let shot = connecting_shot(&mut app, struck);

    let written = app.world_mut().write_message(shot.clone()).is_some();
    assert!(
        written,
        "the raw ShotFired buffer must exist so this guard is not a no-op",
    );
    app.update();

    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "a shot the cursor has not played must pop no hit FCT, got {:?}",
        fct_pops(&mut app),
    );

    play(&mut app, shot);
    fire_with_zero_delta(&mut app);
    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "hit FCT must not appear on the drain frame — that is the shot starting, not landing",
    );

    step_app(&mut app, Duration::from_millis(50), 8);
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "-9", valence_color(FctValence::Damage)),
        "once the played bolt lands the hit FCT must pop, got {pops:?}",
    );
}

#[test]
fn hit_fct_stays_off_until_the_cursor_plays_the_round() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);
    app.insert_resource(ActLog::default());

    let cell = Cell::new(6, 4);
    let level = Level::new(0);
    let struck = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    app.update();

    let shot = connecting_shot(&mut app, struck);
    let shooter = shot.shooter;
    let written = app.world_mut().write_message(shot.clone()).is_some();
    assert!(
        written,
        "the sim writes ShotFired as soon as it resolves — that live buffer is the leak this pins",
    );

    {
        let mut log = app.world_mut().resource_mut::<ActLog>();
        log.append(RecordedAct::new(
            shooter,
            ActProvenance::Commanded,
            ActDeed::Fired {
                target: Some(struck),
                mode:   ModeKind::Single,
                rounds: RoundCount::new(1),
            },
        ));
        log.append(RecordedAct::new(
            shooter,
            ActProvenance::Commanded,
            ActDeed::RoundResolved {
                shot: Box::new(shot),
            },
        ));
    }

    app.world_mut()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(1),
        ));
    app.update();

    let shown = *app.world().resource::<PlaybackCursor>().shown();
    assert_eq!(
        shown, 1,
        "the first tick must play the fire declaration and hold — not the round",
    );
    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "hit FCT must stay off while the cursor is still on the fire declaration, even though \
         the sim already wrote ShotFired, got {:?}",
        fct_pops(&mut app),
    );
}

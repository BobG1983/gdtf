use gdtf_battle_sim::{
    acts::FireRequested,
    ganger::{Direction, Faction},
    metric::CellLevel,
    test_support::SituationBuilder,
};

use super::harness::*;

/// Rounds a shooter is handed, so it never runs dry mid-case.
const LOADED: u16 = 12;

/// The gang the player commands in these cases.
const fn player() -> Faction {
    Faction::new(PLAYER)
}

/// The cell each resolved round stopped in.
fn impacts_of(app: &bevy::app::App) -> Vec<CellLevel> {
    deeds_of(app, "RoundResolved")
        .into_iter()
        .filter_map(|deed| match deed {
            gdtf_battle_sim::act_log::ActDeed::RoundResolved { shot } => {
                Some(CellLevel::new(shot.impact_cell, shot.impact_level))
            }
            _ => None,
        })
        .collect()
}

// Stand a watcher and a shooter, arm the shooter, and fire one round at `aim`.
fn fire_from(
    watch: CellLevel,
    muzzle: CellLevel,
    aim: CellLevel,
) -> (bevy::app::App, bevy::prelude::Entity) {
    let mut app = battle_app(forced_reaction_tuning(0));
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            tough_mover(muzzle, ENEMY, Direction::South),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(shooter) = ganger_at(&mut app, muzzle) else {
        unreachable!("setup spawns the shooter at its fixture cell");
    };
    set_tu_max(&mut app, shooter, u8::MAX);
    set_tu(&mut app, shooter, u8::MAX);
    load_magazine(&mut app, shooter, LOADED);

    let mode = single_mode_of(&mut app, shooter);
    let (cell, level) = aim.split();
    app.world_mut()
        .write_message(FireRequested::new(shooter, mode, cell, level));
    for _ in 0..4 {
        app.update();
    }
    (app, shooter)
}

#[test]
fn a_shot_fired_from_the_dark_across_the_lit_area_is_observed() {
    let watch = ground(5, 5);
    let muzzle = ground(8, 20);
    let crossed = ground(8, 5);
    let (app, shooter) = fire_from(watch, muzzle, ground(8, 0));

    assert!(
        !squad_sees(&app, muzzle),
        "precondition: the shooter stands where the squad cannot see it — {muzzle:?}",
    );
    assert!(
        squad_sees(&app, crossed),
        "precondition: the round's flight must cross a cell the squad IS watching — \
         {crossed:?}",
    );
    for at in impacts_of(&app) {
        assert!(
            !squad_sees(&app, at),
            "precondition: the round must stop where the squad cannot see, so only the \
             crossing lights this entry — {at:?} is lit",
        );
    }

    let fired: Vec<_> = logged_of(&app, "Fired")
        .into_iter()
        .filter(|fact| fact.actor == shooter)
        .collect();
    assert_eq!(
        fired.len(),
        1,
        "the fixture declares exactly one shot: {fired:?}",
    );
    let Some(fact) = fired.first() else {
        unreachable!("the assertion above requires exactly one declaration");
    };
    assert!(
        *fact.witnesses.observed_by(player()),
        "a Fired entry is observed when any cell its rounds crossed was lit, not only the \
         cell the shooter stood on — deed {deed}, recorded {witnesses:?}",
        deed = fact.deed,
        witnesses = fact.witnesses,
    );
}

#[test]
fn a_shot_fired_from_across_and_into_the_dark_is_not_observed() {
    let watch = ground(5, 5);
    let muzzle = ground(40, 40);
    let (app, shooter) = fire_from(watch, muzzle, ground(40, 20));

    for at in [muzzle, ground(40, 30), ground(40, 20)] {
        assert!(
            !squad_sees(&app, at),
            "precondition: nothing on this flight may be lit — {at:?} is",
        );
    }

    let fired: Vec<_> = logged_of(&app, "Fired")
        .into_iter()
        .filter(|fact| fact.actor == shooter)
        .collect();
    assert_eq!(
        fired.len(),
        1,
        "the fixture declares exactly one shot: {fired:?}",
    );
    let Some(fact) = fired.first() else {
        unreachable!("the assertion above requires exactly one declaration");
    };
    assert!(
        !*fact.witnesses.observed_by(player()),
        "a shot fired from, across and into the dark is recorded as unobserved — deed \
         {deed}, recorded {witnesses:?}",
        deed = fact.deed,
        witnesses = fact.witnesses,
    );
}

//! A throw charges exactly what `throw_grenade_tu_cost` quotes.

use bevy::app::App;
use gdtf_battle_sim::{
    acts::{ThrowGrenadeRequested, can_throw_grenade, throw_grenade_tu_cost},
    ganger::{Direction, Tu},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    test_support::SituationBuilder,
    tuning::{
        CombatTuning, EnterEmplacementTu, ExecuteTu, ExitEmplacementTu, OpenDoorTu, ShoveTu,
        StabilizeTu, StanceChangeTu, ThrowTu, TurnTu,
    },
    weapon::{MagazineSize, TrajectoryStyle},
};

use super::harness::*;

fn tuning_of(app: &App) -> CombatTuning {
    app.world()
        .get_resource::<CombatTuning>()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn a_throw_charges_exactly_the_throw_quote() {
    let (mut app, seed) = battle_app(0x5546_0D0D, 1);
    let situation = SituationBuilder::new()
        .with_gangers([thrower(ground(5, 5), Direction::East), target(ground(9, 5))])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(thrower_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the thrower at (5,5)");
    };
    let quoted = throw_grenade_tu_cost(&tuning_of(&app));
    assert!(*quoted > 0, "the throw leaf is a real positive cost");
    let before = app.world().get::<Tu>(thrower_e).map(|tu| **tu);

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(9, 5)));
    step(&mut app, 3);

    assert_eq!(
        before
            .zip(app.world().get::<Tu>(thrower_e).map(|tu| **tu))
            .map(|(b, a)| b - a),
        Some(*quoted),
        "the throw charged exactly what throw_grenade_tu_cost quoted",
    );
}

#[test]
fn throw_grenade_tu_cost_reads_the_throw_leaf() {
    let tuning = CombatTuning {
        throw_tu: ThrowTu::new(23),
        shove_tu: ShoveTu::new(7),
        open_door_tu: OpenDoorTu::new(9),
        enter_emplacement_tu: EnterEmplacementTu::new(11),
        exit_emplacement_tu: ExitEmplacementTu::new(13),
        stance_change_tu: StanceChangeTu::new(15),
        turn_tu: TurnTu::new(17),
        stabilize_tu: StabilizeTu::new(19),
        execute_tu: ExecuteTu::new(21),
        ..CombatTuning::default()
    };

    assert_eq!(
        *throw_grenade_tu_cost(&tuning),
        *tuning.throw_tu,
        "throw_grenade_tu_cost must quote throw_tu and no other TU leaf",
    );
}

#[test]
fn can_throw_grenade_matches_the_gates_the_dispatch_enforces() {
    let tuning = CombatTuning::default();
    let quoted = throw_grenade_tu_cost(&tuning);
    let loaded = Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18));
    let empty = Magazine::new(
        LoadedRounds::new(0),
        MagazineSize::new(4),
        ReloadTu::new(18),
    );
    let pool = Tu::new(100);

    assert!(
        *can_throw_grenade(TrajectoryStyle::Arc, &loaded, &pool, &tuning),
        "a loaded arc weapon with an ample pool may be thrown",
    );
    assert!(
        !*can_throw_grenade(TrajectoryStyle::Straight, &loaded, &pool, &tuning),
        "a straight-trajectory weapon is not throwable",
    );
    assert!(
        !*can_throw_grenade(TrajectoryStyle::Arc, &empty, &pool, &tuning),
        "an empty arc weapon has nothing to throw",
    );
    assert!(
        !*can_throw_grenade(
            TrajectoryStyle::Arc,
            &loaded,
            &Tu::new(quoted.saturating_sub(1)),
            &tuning,
        ),
        "a pool one TU below the quote cannot pay for a throw",
    );
}

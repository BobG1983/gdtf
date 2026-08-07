//! A deliberate shove charges exactly what `shove_tu_cost` quotes.

use gdtf_battle_sim::{
    acts::{ShoveActor, ShoveTarget, can_shove, shove_tu_cost},
    ganger::{Faction, LifeState, Position},
    prelude::{CellLevel, OccupancyGrid},
    surface::SurfaceGrid,
    tuning::{
        CombatTuning, EnterEmplacementTu, ExecuteTu, ExitEmplacementTu, OpenDoorTu, ShoveTu,
        StabilizeTu, StanceChangeTu, ThrowTu, TurnTu,
    },
};

use super::harness::*;

const fn shove_actor(at: CellLevel, faction: u8) -> ShoveActor {
    ShoveActor {
        position: Position::new(at),
        faction:  Faction::new(faction),
    }
}

const fn shove_target(at: CellLevel, faction: u8) -> ShoveTarget {
    ShoveTarget {
        position: Position::new(at),
        faction:  Faction::new(faction),
        life:     LifeState::Alive,
    }
}

#[test]
fn deliberate_shove_charges_exactly_the_shove_quote() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();

    let tuning = app
        .world()
        .get_resource::<CombatTuning>()
        .cloned()
        .unwrap_or_default();
    let quoted = shove_tu_cost(&tuning);
    assert!(
        *quoted > 0,
        "the shipped shove leaf is a real positive cost"
    );
    let tu_before = tu_of(&app, shover);

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        tu_before - tu_of(&app, shover),
        *quoted,
        "the deliberate shove charged exactly what shove_tu_cost quoted",
    );
}

#[test]
fn shove_tu_cost_reads_the_shove_leaf() {
    let tuning = CombatTuning {
        shove_tu: ShoveTu::new(23),
        throw_tu: ThrowTu::new(7),
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
        *shove_tu_cost(&tuning),
        *tuning.shove_tu,
        "shove_tu_cost must quote shove_tu and no other TU leaf",
    );
}

#[test]
fn can_shove_matches_the_gates_the_dispatch_enforces() {
    assert!(
        *can_shove(
            &shove_actor(ground(5, 5), 0),
            &shove_target(ground(6, 5), 1)
        ),
        "an adjacent, hostile, active target is a legal shove",
    );
    assert!(
        !*can_shove(
            &shove_actor(ground(5, 5), 0),
            &shove_target(ground(8, 5), 1)
        ),
        "a non-adjacent target is not a legal shove",
    );
    assert!(
        !*can_shove(
            &shove_actor(ground(5, 5), 0),
            &shove_target(ground(6, 5), 0)
        ),
        "a same-faction ally is not a legal shove",
    );
    let downed = ShoveTarget {
        life: LifeState::Downed,
        ..shove_target(ground(6, 5), 1)
    };
    assert!(
        !*can_shove(&shove_actor(ground(5, 5), 0), &downed),
        "a downed target is not a legal shove",
    );
}

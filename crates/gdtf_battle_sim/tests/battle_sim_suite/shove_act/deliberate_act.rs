use gdtf_battle_sim::{
    prelude::{CellLevel, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

#[test]
fn deliberate_shove_spends_tu_and_deals_no_wound() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();
    let tu_before = tu_of(&app, shover);
    let (hp_before, wounds_before) = (hp_of(&app, target), wounds_of(&app, target));

    shove_and_settle(&mut app, shover, target);

    assert!(
        tu_of(&app, shover) < tu_before,
        "the deliberate shove spends the shover's TU (a TU-costed act)"
    );
    assert_eq!(
        hp_of(&app, target),
        hp_before,
        "the deliberate shove deals NO HP damage (pure displacement)"
    );
    assert_eq!(
        wounds_of(&app, target),
        wounds_before,
        "the deliberate shove records NO Wound (pure displacement — the fall does the harm)"
    );
}

#[test]
fn deliberate_shove_gates_adjacency_and_faction() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let far = shove_ganger(app.world_mut(), ground(8, 5), 1);
    app.update();
    let tu_before = tu_of(&app, shover);
    shove_and_settle(&mut app, shover, far);
    assert_eq!(
        pos_of(&app, far),
        Some(ground(8, 5)),
        "a non-adjacent target is not shoved (the 8-adjacency gate held)"
    );
    assert_eq!(
        tu_of(&app, shover),
        tu_before,
        "a rejected (non-adjacent) shove spends NO TU"
    );

    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let ally = shove_ganger(app.world_mut(), ground(6, 5), 0);
    app.update();
    let tu_before = tu_of(&app, shover);
    shove_and_settle(&mut app, shover, ally);
    assert_eq!(
        pos_of(&app, ally),
        Some(ground(6, 5)),
        "a same-faction ally is not shoved (the opposing-faction gate held)"
    );
    assert_eq!(
        tu_of(&app, shover),
        tu_before,
        "a rejected (ally) shove spends NO TU"
    );
}

#[test]
fn shove_outcomes_are_deterministic_under_same_seed() {
    let run = || -> (Option<CellLevel>, u16) {
        let mut app = shove_app();
        let mut surface = SurfaceGrid::new();
        surface.set_slab(upper(6, 5, 3), SlabState::Present);
        surface.set_slab(upper(7, 5, 1), SlabState::Present);
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let shover = shove_ganger(app.world_mut(), upper(5, 5, 3), 0);
        let target = shove_ganger(app.world_mut(), upper(6, 5, 3), 1);
        app.update();
        let before = hp_of(&app, target);
        shove_and_settle(&mut app, shover, target);
        (pos_of(&app, target), before - hp_of(&app, target))
    };
    assert_eq!(
        run(),
        run(),
        "same seed + same message order must yield identical shove/fall outcomes"
    );
}

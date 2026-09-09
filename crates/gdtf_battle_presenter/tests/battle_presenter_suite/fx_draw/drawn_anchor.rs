use bevy::{app::App, prelude::Text2d, transform::components::Transform};
use gdtf_battle_presenter::{
    DrawnPosition, FctValence, FloatingCombatText, cell_to_world, valence_color,
};
use gdtf_battle_sim::{
    effects::bleed::Bleeding,
    prelude::{BattleInProgress, Cell, CellLevel, Direction, Level, Position, StanceKind},
    test_support::GangerEntityBuilder,
};

use super::{harness::*, probes::*};

fn mirrored_ganger(app: &mut App, at: CellLevel) -> bevy::ecs::entity::Entity {
    GangerEntityBuilder::new()
        .at(at)
        .facing(Direction::North)
        .stance(StanceKind::Standing)
        .aiming(false)
        .tu(60)
        .combat_vitals(10, 2)
        .spawn(app.world_mut())
}

fn pop_xs(app: &mut App, text: &str) -> Vec<f32> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &Transform)>();
    q.iter(app.world())
        .filter(|(_, label, _)| ***label == *text)
        .map(|(_, _, transform)| transform.translation.x)
        .collect()
}

fn drawn_cell(app: &App, ganger: bevy::ecs::entity::Entity) -> Option<CellLevel> {
    app.world()
        .get::<DrawnPosition>(ganger)
        .map(|drawn| *drawn.position())
}

#[test]
fn a_consequence_pop_anchors_at_the_drawn_cell_not_the_cell_the_sim_ran_ahead_to() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let shown = CellLevel::new(Cell::new(3, 6), Level::new(0));
    let ahead = CellLevel::new(Cell::new(17, 6), Level::new(0));
    let ganger = mirrored_ganger(&mut app, shown);

    app.update();
    assert_eq!(
        drawn_cell(&app, ganger),
        Some(shown),
        "the mirror must be seeded at the shown cell for this test to mean anything",
    );

    app.world_mut()
        .entity_mut(ganger)
        .insert(Position::new(ahead));
    assert_eq!(
        drawn_cell(&app, ganger),
        Some(shown),
        "the drawn mirror must NOT follow the live move — that lag is the situation under test",
    );

    play(&mut app, Bleeding::new(ganger));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "Bleeding", valence_color(FctValence::Status)),
        "the played bleed must still pop its tag, got {pops:?}",
    );

    let (shown_cell, shown_level) = shown.split();
    let (ahead_cell, ahead_level) = ahead.split();
    let drawn_x = cell_to_world(shown_cell, shown_level).x;
    let ahead_x = cell_to_world(ahead_cell, ahead_level).x;
    assert!(
        (drawn_x - ahead_x).abs() > 1.0,
        "the two cells must be far apart for the assertions below to discriminate",
    );
    let xs = pop_xs(&mut app, "Bleeding");
    assert!(
        xs.iter().any(|x| (x - drawn_x).abs() < 0.001),
        "the pop must sit on the DRAWN cell x ({drawn_x}), got {xs:?}",
    );
    assert!(
        !xs.iter().any(|x| (x - ahead_x).abs() < 0.001),
        "no pop may sit on the cell the sim ran ahead to ({ahead_x}), got {xs:?}",
    );
}

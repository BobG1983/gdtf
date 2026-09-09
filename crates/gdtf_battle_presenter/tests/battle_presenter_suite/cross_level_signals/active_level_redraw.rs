use bevy::{
    platform::collections::HashSet,
    prelude::{App, Transform, With},
};
use gdtf_battle_presenter::{
    ActiveLevel, CrossLevelBadgeKind, CrossLevelBadgeTile, CrossLevelSignals, Layer, LevelDelta,
    cell_to_world_layered,
};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink, build_vertical_link_graph},
    visibility::SquadVisibility,
};

use super::harness::{settle, signals_app};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

fn drawn_tile_zs(app: &mut App) -> Vec<f32> {
    let mut query = app
        .world_mut()
        .query_filtered::<&Transform, With<CrossLevelBadgeTile>>();
    query
        .iter(app.world())
        .map(|transform| transform.translation.z)
        .collect()
}

#[test]
fn active_level_change_alone_redraws_the_badge_at_the_new_storeys_z_band() {
    let mut app = signals_app();

    let cell = Cell::new(5, 5);
    let a_lower = key(5, 5, 2);
    let a_upper = key(5, 5, 5);
    let b_lower = key(5, 5, 3);
    let b_upper = key(5, 5, 6);

    let situation = SituationBuilder::new()
        .slab_at(a_lower)
        .slab_at(a_upper)
        .slab_at(b_lower)
        .slab_at(b_upper)
        .vertical_link(VerticalLink::new(a_lower, a_upper, LinkKind::stair()))
        .vertical_link(VerticalLink::new(b_lower, b_upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    let Ok(graph) = result else {
        return;
    };
    app.world_mut().insert_resource(graph);

    let explored: HashSet<CellLevel> = [a_lower, a_upper, b_lower, b_upper].into_iter().collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(explored.clone(), explored));

    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(2)));
    settle(&mut app);

    let expected_badge = CrossLevelBadgeKind::ConnectorDelta {
        delta: LevelDelta::new(3),
    };
    let badges_at_2 = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(cell)
        .to_vec();
    assert_eq!(
        badges_at_2,
        vec![expected_badge],
        "storey 2 must surface link A's ConnectorDelta(+3), got {badges_at_2:?}",
    );
    let tile_zs_lower = drawn_tile_zs(&mut app);
    assert_eq!(
        tile_zs_lower.len(),
        1,
        "exactly one badge tile must be drawn"
    );
    let drawn_z_lower = tile_zs_lower[0];
    assert!(
        (drawn_z_lower - cell_to_world_layered(cell, Level::new(2), Layer::CrossLevelSignal).z)
            .abs()
            < f32::EPSILON,
        "the badge must draw at storey 2's Z-band",
    );

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(3));
    settle(&mut app);

    let badges_at_3 = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(cell)
        .to_vec();
    assert_eq!(
        badges_at_3, badges_at_2,
        "storey 3's derived badge must be IDENTICAL to storey 2's (the contrived \
         same-content-different-cause scenario this regression needs), got {badges_at_3:?}",
    );

    let tile_zs_upper = drawn_tile_zs(&mut app);
    assert_eq!(
        tile_zs_upper.len(),
        1,
        "still exactly one badge tile must be drawn after the level switch",
    );
    let drawn_z_upper = tile_zs_upper[0];
    assert!(
        (drawn_z_upper - cell_to_world_layered(cell, Level::new(3), Layer::CrossLevelSignal).z)
            .abs()
            < f32::EPSILON,
        "the badge must redraw at storey 3's Z-band even though CrossLevelSignals \
         content never changed — a redraw gated on CrossLevelSignals alone would \
         leave it stuck at storey 2's Z ({drawn_z_lower}), got {drawn_z_upper}",
    );
    assert!(
        (drawn_z_upper - drawn_z_lower).abs() > f32::EPSILON,
        "storey 2 and storey 3 must draw at DIFFERENT Z-bands",
    );
}

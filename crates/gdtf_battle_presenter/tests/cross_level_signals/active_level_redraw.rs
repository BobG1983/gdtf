//! Regression (gate finding, GTW-596): [`draw_cross_level_signals`] must redraw
//! when [`ActiveLevel`] changes even on the frame where the newly-derived
//! [`CrossLevelSignals`] happens to be byte-identical to the old storey's set —
//! a badge's drawn Z-band is hard-cut to the active storey
//! ([`cell_to_world_layered`]), so gating the redraw on
//! `CrossLevelSignals::is_changed()` ALONE would leave a stale badge drawn at
//! the OLD storey's Z-band once the view has actually moved.
//!
//! Two INDEPENDENT stair links are stacked at the SAME `(x, y) = (5, 5)`
//! column — link A spans storeys 2 <-> 5, link B spans storeys 3 <-> 6 — so
//! each contributes the IDENTICAL `ConnectorDelta(+3)` badge at the SAME cell
//! from its OWN lower endpoint: viewing storey 2 surfaces link A's lower
//! endpoint; viewing storey 3 surfaces link B's lower endpoint. `CrossLevelSignals`
//! therefore never ticks `Changed` across that level switch (the derived map is
//! equal both times), proving the redraw trigger genuinely needs the
//! `ActiveLevel` input, not just `CrossLevelSignals`.

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

/// Every currently-drawn [`CrossLevelBadgeTile`]'s world Z — this fixture only
/// ever surfaces ONE badge cell at a time, so a correct draw always yields
/// exactly one.
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
    // Link A: the lower endpoint sits on storey 2; viewed from storey 2 it
    // contributes ConnectorDelta(+3) at `cell`.
    let a_lower = key(5, 5, 2);
    let a_upper = key(5, 5, 5);
    // Link B: an entirely independent link whose lower endpoint sits on storey
    // 3, at the SAME `(x, y)`; viewed from storey 3 it contributes the SAME
    // ConnectorDelta(+3) at `cell`.
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

    // Phase 1 — active storey 2: link A's lower endpoint is on-storey.
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

    // Phase 2 — switch the active storey to 3: link B's lower endpoint is now
    // on-storey, deriving the SAME `ConnectorDelta(+3)` badge at the SAME cell —
    // `CrossLevelSignals` must be byte-identical to phase 1's (never ticks
    // `Changed`), yet the drawn Z-band must follow the new active storey.
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

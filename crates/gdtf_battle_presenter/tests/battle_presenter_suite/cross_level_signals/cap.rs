use bevy::{platform::collections::HashSet, prelude::App};
use gdtf_battle_presenter::{
    ActiveLevel, CrossLevelBadgeKind, CrossLevelSignals, LevelDelta, TerrainSprite, ThreatCount,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    falls::StoreysFallen,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position},
    surface::{SlabState, SurfaceGrid},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink, build_vertical_link_graph},
    visibility::SquadVisibility,
};

use super::harness::{settle, signals_app, visible_label_texts, visible_tile_count};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

fn spawn_enemy(app: &mut App, at: CellLevel) {
    app.world_mut()
        .spawn((Position::new(at), Faction::new(1), LifeState::Alive));
}

#[test]
fn two_enemies_sharing_a_delta_draw_one_capped_count_pip_badge() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let enemy_cell_level = key(2, 2, 2);
    let visible: HashSet<CellLevel> = std::iter::once(enemy_cell_level).collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));

    spawn_enemy(&mut app, enemy_cell_level);
    spawn_enemy(&mut app, enemy_cell_level);

    settle(&mut app);

    let cell = Cell::new(2, 2);
    let badges = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(cell)
        .to_vec();
    assert_eq!(
        badges,
        vec![CrossLevelBadgeKind::Threat {
            delta: LevelDelta::new(2),
            count: ThreatCount::ONE.incremented(),
        }],
        "two enemies sharing delta +2 on one cell must collapse into ONE x2 badge, got {badges:?}",
    );
    assert_eq!(
        visible_tile_count(&mut app),
        1,
        "the draw system must render exactly ONE tile for the deduped badge, never two",
    );
    assert_eq!(
        visible_label_texts(&mut app),
        vec!["+2 x2".to_string()],
        "the drawn label must carry the count pip",
    );
}

#[test]
fn four_candidates_across_producers_draw_only_the_top_three_in_priority_order() {
    let mut app = signals_app();
    let active_level = Level::new(3);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));

    let cell = Cell::new(5, 5);
    let near_enemy = key(5, 5, 2);
    let far_enemy = key(5, 5, 6);
    let hole = key(5, 5, 3);
    let floor = key(6, 5, 3);

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.set_slab(hole, SlabState::Absent);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().spawn(TerrainSprite { at: floor });

    let upper = key(5, 5, 4);
    let (situation, _placements) = SituationBuilder::new()
        .slab_at(hole)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(hole, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation.map);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    let Ok(graph) = result else {
        return;
    };
    app.world_mut().insert_resource(graph);

    let explored: HashSet<CellLevel> = [near_enemy, far_enemy, hole].into_iter().collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(explored.clone(), explored));

    spawn_enemy(&mut app, near_enemy);
    spawn_enemy(&mut app, far_enemy);

    settle(&mut app);

    let badges = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(cell)
        .to_vec();
    assert_eq!(
        badges,
        vec![
            CrossLevelBadgeKind::Threat {
                delta: LevelDelta::new(-1),
                count: ThreatCount::ONE,
            },
            CrossLevelBadgeKind::Threat {
                delta: LevelDelta::new(3),
                count: ThreatCount::ONE,
            },
            CrossLevelBadgeKind::DropDepth {
                storeys: StoreysFallen::new(3),
            },
        ],
        "4 candidates (2 Threat + 1 DropDepth + 1 ConnectorDelta) must cap at the \
         top 3 by priority — the ConnectorDelta(+1) candidate is silently dropped, \
         got {badges:?}",
    );

    assert_eq!(
        visible_tile_count(&mut app),
        3,
        "the draw system must render exactly the capped 3 tiles, never the raw 4 candidates",
    );
    let mut labels = visible_label_texts(&mut app);
    labels.sort();
    let mut expected = vec!["+3".to_string(), "-1".to_string(), "v3".to_string()];
    expected.sort();
    assert_eq!(
        labels, expected,
        "the drawn labels must be exactly the capped 3 (Threat -1, Threat +3, DropDepth v3) \
         — the dropped ConnectorDelta's \"+1\" label must never be drawn, got {labels:?}",
    );
}

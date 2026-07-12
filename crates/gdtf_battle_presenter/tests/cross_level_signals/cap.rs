//! Aggregation + cap regression coverage (GTW-596 acceptance clause 3, the
//! RESOLVED SPEC) proven through the REAL registered `derive_cross_level_signals`
//! and `draw_cross_level_signals` systems — the sibling in-crate `test/cap.rs`
//! pins the pure `aggregate_threats` / `cap_badges` / `build_signals` helpers
//! directly; these prove the SAME dedupe + cap rules hold once wired through
//! live Bevy resources/queries, and that the drawn tile/label POOL itself
//! reflects the capped set — never the raw candidate count.

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

/// Spawn a bare enemy ganger — the derive system's Query only reads these three
/// components (the `threat.rs` precedent).
fn spawn_enemy(app: &mut App, at: CellLevel) {
    app.world_mut()
        .spawn((Position::new(at), Faction::new(1), LifeState::Alive));
}

/// Acceptance clause 3: two squad-VISIBLE enemies sharing the SAME level-delta on
/// one cell collapse into ONE drawn badge with an `x2` count pip — proven through
/// the REAL registered derive + draw systems (not just the pure `aggregate_threats`
/// helper `test/cap.rs` pins): exactly one tile, exactly one label reading `"+2 x2"`.
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

/// Acceptance clause 3: MORE than the 3-badge cap's worth of candidates on one
/// cell, spanning all three producers (2 Threat + 1 `DropDepth` + 1
/// `ConnectorDelta` = 4), keep only the top-priority 3 (Threat nearest-first, then
/// `DropDepth`, then `ConnectorDelta`) — proven DRAWN through the real derive +
/// draw pipeline (not just the pure `cap_badges` helper `test/cap.rs` pins):
/// exactly 3 tiles, exactly 3 labels, and the dropped `ConnectorDelta`'s `"+1"`
/// label never appears.
#[test]
fn four_candidates_across_producers_draw_only_the_top_three_in_priority_order() {
    let mut app = signals_app();
    let active_level = Level::new(3);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));

    let cell = Cell::new(5, 5);
    let near_enemy = key(5, 5, 2); // delta -1 (magnitude 1)
    let far_enemy = key(5, 5, 6); // delta +3 (magnitude 3)
    let hole = key(5, 5, 3); // same active storey — the DropDepth AND ConnectorDelta candidate
    let floor = key(6, 5, 3); // drawn neighbour, supplies the "built footprint"

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.destroy_slab(hole);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().spawn(TerrainSprite { at: floor });

    // A scratch `Situation` purely to hand `build_vertical_link_graph` a VALID
    // graph to validate — deliberately unrelated to the `SurfaceGrid` resource
    // above (the derive system reads the two independently, `derive.rs`'s
    // documented decoupling): the live world has `hole` as a destroyed slab while
    // the scratch situation has it as a normal stair landing, and neither
    // producer cross-checks the other.
    let upper = key(5, 5, 4);
    let situation = SituationBuilder::new()
        .slab_at(hole)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(hole, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation);
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

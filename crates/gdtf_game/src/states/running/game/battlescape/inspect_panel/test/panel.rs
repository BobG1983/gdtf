use bevy::{
    MinimalPlugins,
    ecs::system::{RunSystemOnce, SystemState},
    platform::collections::HashSet,
    prelude::*,
    scene::ScenePlugin,
    ui::{Display, Node, PositionType, Val},
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_input::InspectTarget;
use gdtf_battle_presenter::ShownSquadVisibility;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::PlayerFaction,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Hp, HpMax, Stance, StanceKind, Tu, TuMax, Wounds},
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Faction, Level, OccupancyGrid},
    visibility::SquadVisibility,
};
use gdtf_ui::theme::default_theme;

use crate::states::running::game::battlescape::inspect_panel::{
    components::{InspectObjectBlock, InspectPanelRoot, InspectStatBlockHost},
    shadow::{ShownCoverLedger, ShownEmplacements, ShownOccupancyGrid},
    systems::{spawn_inspect_panel, update_inspect_panel},
};

type SpawnParams<'w, 's> = SystemState<(
    Commands<'w, 's>,
    Option<Res<'w, gdtf_ui::theme::GdtfTheme>>,
    Option<Res<'w, gdtf_battle_presenter::TopDownAtlases>>,
)>;

const PLAYER: Faction = Faction::new(0);

fn panel_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.insert_resource(default_theme());
    let world = app.world_mut();

    let mut state: SpawnParams = SystemState::new(world);
    if let Ok((commands, theme, atlases)) = state.get(world) {
        spawn_inspect_panel(commands, theme, atlases);
        state.apply(world);
    }
    app.update();
    app
}

fn spawn_root_node() -> Option<Node> {
    let mut app = panel_app();
    let world = app.world_mut();
    let mut roots = world.query_filtered::<&Node, With<InspectPanelRoot>>();
    roots.iter(world).next().cloned()
}

fn node_of<M: Component>(world: &mut World) -> Option<Node> {
    let mut marked = world.query_filtered::<&Node, With<M>>();
    marked.iter(world).next().cloned()
}

fn entity_of<M: Component>(world: &mut World) -> Option<Entity> {
    let mut marked = world.query_filtered::<Entity, With<M>>();
    marked.iter(world).next()
}

fn cell(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

// A player ganger standing in cover, with every shadow the panel reads pinned to its cell.
fn a_ganger_in_cover(app: &mut App, at: CellLevel) {
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER,
            Stance::new(StanceKind::Standing),
            Tu::new(5),
            TuMax::new(10),
            Hp::new(8),
            HpMax::new(10),
            Wounds::new(0),
        ))
        .id();

    let mut grid = OccupancyGrid::new();
    grid.set_occupant(at, Some(ganger));
    grid.set_terrain(at, TerrainKind::Cover);
    let mut shown_grid = ShownOccupancyGrid::default();
    shown_grid.promote(&grid);

    let mut ledger = CoverLedger::new();
    ledger.insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(10),
            HeightBand::Mid,
            ArmorProtection::new(2),
            ArmorHardness::new(1),
            TerrainPieceKind::Cover,
        ),
    );
    let mut shown_ledger = ShownCoverLedger::default();
    shown_ledger.promote(&ledger);

    let seen: HashSet<CellLevel> = std::iter::once(at).collect();
    let mut shown_fog = ShownSquadVisibility::default();
    shown_fog.promote(&SquadVisibility::new(seen.clone(), seen));

    let mut target = InspectTarget::new(None);
    target.set_pinned(at);

    app.insert_resource(target);
    app.insert_resource(shown_grid);
    app.insert_resource(shown_ledger);
    app.insert_resource(ShownEmplacements::default());
    app.insert_resource(shown_fog);
    app.insert_resource(PlayerFaction::new(PLAYER));
}

#[test]
fn inspect_panel_root_is_an_absolute_overlay() {
    let node = spawn_root_node();
    assert!(
        node.is_some(),
        "spawn_inspect_panel must spawn an InspectPanelRoot"
    );
    let Some(node) = node else {
        return;
    };
    assert_eq!(
        node.position_type,
        PositionType::Absolute,
        "the inspect panel root is an absolute overlay (contributes nothing to the viewport inset)",
    );
}

#[test]
fn inspect_panel_root_width_is_a_fixed_viewport_fraction() {
    let node = spawn_root_node();
    assert!(
        node.is_some(),
        "spawn_inspect_panel must spawn an InspectPanelRoot"
    );
    let Some(node) = node else {
        return;
    };
    assert!(
        matches!(node.width, Val::Vw(_)),
        "the inspect panel width must be a fixed viewport-width fraction (Val::Vw), got {:?}",
        node.width,
    );
    assert!(
        matches!(node.max_height, Val::Vh(_)),
        "the inspect panel max-height must be a viewport-height fraction (Val::Vh), got {:?}",
        node.max_height,
    );
}

#[test]
fn inspect_panel_root_is_anchored_top_right() {
    let node = spawn_root_node();
    assert!(
        node.is_some(),
        "spawn_inspect_panel must spawn an InspectPanelRoot"
    );
    let Some(node) = node else {
        return;
    };
    assert!(
        matches!(node.top, Val::Vh(_)),
        "the inspect panel is anchored to the top (Val::Vh), got {:?}",
        node.top,
    );
    assert!(
        matches!(node.right, Val::Vw(_)),
        "the inspect panel is anchored to the right (Val::Vw), got {:?}",
        node.right,
    );
    assert_eq!(
        node.left,
        Val::Auto,
        "the top-right anchor leaves `left` auto (not a left anchor)",
    );
}

#[test]
fn a_ganger_in_cover_leaves_both_blocks_drawn() {
    let mut app = panel_app();
    a_ganger_in_cover(&mut app, cell(3, 3));

    let ran = app.world_mut().run_system_once(update_inspect_panel);
    assert!(ran.is_ok(), "the panel update must run in this fixture");

    assert_eq!(
        node_of::<InspectStatBlockHost>(app.world_mut()).map(|node| node.display),
        Some(Display::Flex),
        "the cell holds a ganger the squad can see, so its stat block is drawn",
    );
    assert_eq!(
        node_of::<InspectObjectBlock>(app.world_mut()).map(|node| node.display),
        Some(Display::Flex),
        "the same cell holds cover, and the panel no longer hides one block to draw the other",
    );
}

#[test]
fn the_panel_stacks_its_two_blocks_rather_than_sharing_one_box() {
    let mut app = panel_app();
    let world = app.world_mut();
    let root = entity_of::<InspectPanelRoot>(world);
    let host = entity_of::<InspectStatBlockHost>(world);
    let object_block = entity_of::<InspectObjectBlock>(world);
    let (Some(root), Some(host), Some(object_block)) = (root, host, object_block) else {
        unreachable!("the panel spawns a root, a stat-block host and an object block");
    };

    assert_eq!(
        node_of::<InspectPanelRoot>(world).map(|node| node.flex_direction),
        Some(FlexDirection::Column),
        "the root is a column, so the two blocks stack one under the other",
    );
    let children: Vec<Entity> = world
        .get::<Children>(root)
        .map(|kids| kids.iter().collect())
        .unwrap_or_default();
    assert!(
        children.contains(&host) && children.contains(&object_block),
        "both blocks are children of that column: {children:?}",
    );
    for child in [host, object_block] {
        assert_eq!(
            world.get::<Node>(child).map(|node| node.position_type),
            Some(PositionType::Relative),
            "a block laid out relative takes its own row in the column instead of \
             overlapping the other",
        );
    }
}

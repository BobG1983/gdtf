use bevy::{input::ButtonInput, prelude::*, scene::ScenePlugin};
use cobalt_test_utils::{press_left, unwatched_asset_plugin};
use gdtf_battle_input::GdtfBattleInputPlugin;
use gdtf_battle_presenter::{ActiveLevel, ViewMode, cell_to_world};
// ---------------------------------------------------------------------------------
use gdtf_battle_sim::{
    armor::ArmorFloor,
    armor::ArmorHardness,
    armor::ArmorIntegrity,
    armor::ArmorName,
    armor::ArmorPiece,
    armor::ArmorProtection,
    armor::ArmorRegistry,
    armor::ArmorSpec,
    armor::ArmorType,
    ganger::Aim,
    ganger::Aiming,
    ganger::Direction,
    ganger::Facing,
    ganger::GangerName,
    ganger::Luck,
    ganger::Stance,
    ganger::StanceKind,
    ganger::Toughness,
    rng::BattleSeed,
    situation::GangerSpawn,
    situation::PlacedGanger,
    situation::Situation,
    test_support::{setup_request, test_weapon_spec},
    tuning::CombatTuning,
    weapon::WeaponName,
    weapon::WeaponRegistry,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, PlayerFaction},
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level},
};

use super::harness::*;

const REAL_FLOW_WEAPON_KEY: &str = "test-weapon";

const REAL_FLOW_ARMOR_KEY: &str = "test-armor";

fn real_flow_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(REAL_FLOW_WEAPON_KEY.to_owned()),
        test_weapon_spec(),
    )])
}

const fn real_flow_armor(base: i32) -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

fn real_flow_armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(REAL_FLOW_ARMOR_KEY.to_owned()),
        real_flow_armor(1),
    )])
}

fn real_flow_ganger(at: CellLevel, faction: u8) -> GangerSpawn {
    use gdtf_battle_sim::test_support::GangerSpawnBuilder;
    GangerSpawnBuilder::new()
        .at(at)
        .name(GangerName::new(format!("Ganger {faction}")))
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::East))
        .stance(Stance::new(StanceKind::Crouching))
        .aiming(Aiming::new(true))
        .aim(Aim::new(f32::from(faction) + 2.0))
        .toughness(Toughness::new(f32::from(faction) + 3.0))
        .luck(Luck::new(f32::from(faction) + 1.0))
        .build()
}

fn real_flow_situation() -> (Situation, Vec<PlacedGanger>) {
    use gdtf_battle_sim::test_support::SituationBuilder;
    let level = Level::new(0);
    SituationBuilder::new()
        .with_gangers(vec![
            real_flow_ganger(CellLevel::new(Cell::new(5, 6), level), 0),
            real_flow_ganger(CellLevel::new(Cell::new(7, 8), level), 1),
        ])
        .build()
}

fn real_flow_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(BattleSimPlugin);
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(real_flow_registry());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_melee_weapon_registry());
    app.world_mut().insert_resource(real_flow_armor_registry());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_gang_registry());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

fn request_setup(app: &mut App, built: (Situation, Vec<PlacedGanger>)) {
    app.world_mut()
        .write_message(setup_request(built, BattleSeed::new(0x5A1C)));
}

fn selected_faction(app: &mut App) -> Option<Faction> {
    let entity = selected(app)?;
    let world = app.world_mut();
    let mut query = world.query::<&Faction>();
    query.get(world, entity).ok().copied()
}

#[test]
fn real_setup_flow_auto_selects_a_player_ganger_same_update() {
    let mut app = real_flow_app();

    assert_eq!(
        selected(&app),
        None,
        "precondition: nothing selected before the battle is set up",
    );

    request_setup(&mut app, real_flow_situation());

    app.update();

    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the real setup must have inserted BattleInProgress from the SetupBattleRequested",
    );
    assert_eq!(
        app.world().get_resource::<PlayerFaction>().map(|p| **p),
        Some(Faction::new(0)),
        "the real setup must have seeded PlayerFaction(0) from the situation default",
    );

    assert!(
        selected(&app).is_some(),
        "a player ganger must be auto-selected on the SAME update the real setup runs — NOT \
         left 'No ganger selected' (A2 regression: the missing setup->Gather sync point)",
    );
    assert_eq!(
        selected_faction(&mut app),
        Some(Faction::new(0)),
        "the auto-selected ganger must be a PLAYER-faction ganger, never the enemy",
    );
}

#[test]
fn selection_highlight_tracks_selected_ganger_not_empty_cells() {
    let level = Level::new(0);
    let mut app = real_flow_app();
    request_setup(&mut app, real_flow_situation());
    for _ in 0..4 {
        app.update();
    }

    let Some(selected_entity) = selected(&app) else {
        unreachable!("A2 must have auto-selected a player ganger before checking the highlight");
    };
    let ganger_cell = Cell::new(5, 6);

    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one selection-highlight sprite exists once a ganger is selected",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(ganger_cell, level), Visibility::Visible)),
        "the selection highlight must sit on the SELECTED ganger's cell (5, 6), not elsewhere",
    );

    let empty_cell = CellLevel::new(Cell::new(20, 20), level);
    set_hovered(&mut app, Some(empty_cell));
    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(selected_entity),
        "a bare-floor click must not change the selection (MOVE, not re-select)",
    );
    let world_at_empty = cell_to_world(Cell::new(20, 20), level);
    let highlight = highlight_state(&mut app);
    assert!(
        highlight.is_some_and(|(t, _)| t != world_at_empty),
        "the selection marker must NOT move onto the clicked bare-floor cell (20, 20) — \
         it tracks the selected ganger, never an empty cell (B)",
    );
}

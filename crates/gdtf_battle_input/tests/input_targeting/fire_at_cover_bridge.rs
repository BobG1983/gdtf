//! Click cover to fire: shot depletes and frees the cell.
use bevy::{input::ButtonInput, platform::collections::HashSet, prelude::*, scene::ScenePlugin};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, FireTargetHighlight, ViewMode};
use gdtf_battle_sim::{
    acts::SimActsPlugin,
    armor::{ArmorHardness, ArmorProtection},
    battle::PlayerFaction,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    occupancy::TerrainKind,
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, Stance,
        StanceKind, Tu,
    },
    rng::BattleSeed,
    test_support::{insert_sim_resources, single_mode},
    visibility::SquadVisibility,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

const PLAYER_FACTION: Faction = Faction::new(1);
const LEVEL: Level = Level::new(0);

fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), LEVEL)
}

fn cover_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), LEVEL)
}

fn bridge_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    insert_sim_resources(&mut app, BattleSeed::new(0xC0BA_17C0));
    app.insert_resource(gdtf_battle_sim::battle::BattleInProgress);
    app.insert_resource(ActiveLevel::new(LEVEL));
    app.insert_resource(ViewMode::default());
    app.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(FireTargetHighlight::cleared());
    app
}

fn spawn_shooter(app: &mut App) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("breacher".to_owned()),
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(200),
            WeaponPunch::new(80),
            WeaponShred::new(40),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![single_mode(0.2, 1)]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let shooter = app
        .world_mut()
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(250),
            TuMax::new(100),
            PLAYER_FACTION,
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(shooter_cell(), Some(shooter));
    shooter
}

const fn low_hp_cover() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

fn mark_visible(app: &mut App, cell: CellLevel) {
    let mut visible: HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
}

#[test]
fn clicking_cover_fires_and_the_sim_depletes_it() {
    let mut app = bridge_app();
    let mut occupancy = OccupancyGrid::new();
    occupancy.set_terrain(cover_cell(), TerrainKind::Cover);
    app.insert_resource(occupancy);
    let mut cover = CoverLedger::new();
    cover.insert(cover_cell(), low_hp_cover());
    app.insert_resource(cover);

    let shooter = spawn_shooter(&mut app);
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    mark_visible(&mut app, cover_cell());

    {
        let grid = app.world().resource::<OccupancyGrid>();
        assert!(
            *grid.is_blocked(&cover_cell()),
            "BEFORE: the standing cover cell must block",
        );
        assert!(
            !*grid.is_cover_destroyed(&cover_cell()),
            "BEFORE: the cover cell must not yet be in the destroyed-cover set",
        );
    }

    app.world_mut()
        .insert_resource(InspectTarget::new(Some(cover_cell())));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    app.update();

    let grid = app.world().resource::<OccupancyGrid>();
    assert!(
        *grid.is_cover_destroyed(&cover_cell()),
        "AFTER: clicking the cover must have fired a real shot that depleted it — \
         sync_destroyed_cover marks the smashed cell destroyed (the click→fire→free chain); \
         got destroyed-set miss",
    );
    assert!(
        !*grid.is_blocked(&cover_cell()),
        "AFTER: a destroyed cover cell must stop blocking (the freed cell)",
    );
}

//! window), armed shooter / enemy authoring, and the cursor drive.

use bevy::{
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    input::ButtonInput,
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_battle_sim::{
    acts::SimActsPlugin,
    cover::HeightBand,
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        BattleInProgress, Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid,
        Position, Stance, StanceKind, Tu,
    },
    rng::BattleSeed,
    test_support::{TEST_SEED, insert_sim_resources, single_mode},
    visibility::SquadVisibility,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FightMode, FightModeKind,
        FightModeSpec, FireMode, Handedness, HandlingProfile, Kickback, MagazineSize,
        MeleeDamageProfile, MeleeWeaponBundle, Reach, Shove, Stable, Strikes, TuCost, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);

pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);
pub(crate) const SHOOTER_CURSOR_OFFSET: Vec2 = Vec2::new(40.0, 32.0);
pub(crate) const TARGET_CURSOR_OFFSET: Vec2 = Vec2::new(200.0, 160.0);

pub(crate) fn synthetic_camera() -> Camera {
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(TARGET_SIZE.x, TARGET_SIZE.y);
    Camera {
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: TARGET_SIZE.as_uvec2(),
                scale_factor:  1.0,
            }),
            clip_from_view: projection.get_clip_from_view(),
            ..ComputedCameraValues::default()
        },
        ..Camera::default()
    }
}

pub(crate) fn endtoend_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(bevy::scene::ScenePlugin)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    insert_sim_resources(&mut app, BattleSeed::new(TEST_SEED));
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());

    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        synthetic_camera(),
        GlobalTransform::IDENTITY,
        Projection::Orthographic(OrthographicProjection::default_2d()),
        Frustum::default(),
    ));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

pub(crate) fn spawn_armed_shooter(app: &mut App, cell: CellLevel, facing: Direction) -> Entity {
    spawn_armed_shooter_inner(app, cell, facing, false)
}

pub(crate) fn spawn_armed_shooter_melee_first(
    app: &mut App,
    cell: CellLevel,
    facing: Direction,
) -> Entity {
    spawn_armed_shooter_inner(app, cell, facing, true)
}

pub(crate) fn spawn_armed_shooter_inner(
    app: &mut App,
    cell: CellLevel,
    facing: Direction,
    melee_first: bool,
) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("probe-weapon".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
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
            PLAYER_FACTION,
            Position::new(cell),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Shooting::new(1.0),
            Tu::new(200),
            TuMax::new(100),
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
    if melee_first {
        let melee = MeleeWeaponBundle::new(
            WeaponName::new("probe-melee".to_owned()),
            MeleeDamageProfile::new(
                WeaponDamage::new(9),
                WeaponPunch::new(3),
                WeaponShred::new(8),
                DamageType::Rend,
            ),
            FatalBias::new(4.0),
            Handedness::OneHanded,
            Reach::new(1),
            FightMode::new(vec![FightModeSpec::new(
                FightModeKind::Swing,
                TuCost::new(20),
                Strikes::new(1),
            )]),
            Shove::new(false),
        );
        app.world_mut().spawn((WieldedBy::new(shooter), melee));
    }
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    shooter
}

pub(crate) fn place_armed_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app
        .world_mut()
        .spawn((
            ENEMY_FACTION,
            Hp::new(30),
            Wounds::new(6),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(1.0),
            Luck::new(0.0),
        ))
        .id();
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(cell, Some(enemy));
        grid.set_occupant_band(cell, Some(HeightBand::High));
    }
    let mut visible: bevy::platform::collections::HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
    enemy
}

pub(crate) fn hover_at(app: &mut App, offset: Vec2) -> CellLevel {
    set_cursor(app, Some(TARGET_SIZE * 0.5 + offset));
    app.update();
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .unwrap_or_else(|| CellLevel::new(Cell::new(0, 0), Level::new(0)))
}

pub(crate) fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

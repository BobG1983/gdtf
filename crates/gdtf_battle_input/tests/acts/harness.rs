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
use cobalt_test_utils::{MessageProbePlugin, clear_mouse, press_left, probed};
use gdtf_battle_input::{
    BoundKey, ChosenFireMode, GdtfBattleInputPlugin, InspectTarget, Keybinds, chosen_spec,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
        ExitEmplacementRequested, FireRequested, MoveRequested, OpenDoorRequested, ReloadRequested,
        SetAimingRequested, SetFacingRequested, SetStanceRequested, SimActsPlugin,
        StabilizeDownedRequested,
    },
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{
        BattleInProgress, Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid,
        StanceKind,
    },
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, TEST_SEED, fight_rng, insert_sim_resources, wield},
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, MeleeWeapon, ModeConeMult, ModeKind,
        ModeShots, ModeTuPercent, Wields,
    },
};

pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);

pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

pub(crate) const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

pub(crate) const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

pub(crate) fn sbf_selector() -> FireMode {
    FireMode::new(vec![
        spec(ModeKind::Single, 0.2, 1),
        spec(ModeKind::Burst, 0.4, 3),
        spec(ModeKind::Full, 0.7, 6),
    ])
}

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

pub(crate) fn acts_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(bevy::scene::ScenePlugin)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(TEST_SEED));
    let level = Level::new(0);
    app.world_mut().insert_resource(ActiveLevel::new(level));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(fight_rng(TEST_SEED));
    app.world_mut().insert_resource(test_keybinds());
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

pub(crate) fn armed_ganger(
    app: &mut App,
    selector: FireMode,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .faction(PLAYER_FACTION)
        .stance(stance)
        .facing(facing)
        .aiming(false)
        .life_state(LifeState::Alive)
        .tu(255)
        .tu_max(100)
        .spawn(app.world_mut());
    wield(
        app.world_mut(),
        ganger,
        (
            selector,
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            Handedness::OneHanded,
        ),
    );
    ganger
}

/// The ranged weapon `ganger` holds, which every armed fixture here gives it exactly one of.
pub(crate) fn gun_of(app: &App, ganger: Entity) -> Option<Entity> {
    let wields = app.world().get::<Wields>(ganger)?;
    wields.ranged_weapon(|weapon| {
        app.world()
            .get_entity(weapon)
            .is_ok_and(|row| row.contains::<MeleeWeapon>())
    })
}

/// Pick `kind` for `ganger`, the way the mode panel picks it: on the gun it holds.
pub(crate) fn pick_mode(app: &mut App, ganger: Entity, kind: ModeKind) {
    let Some(weapon) = gun_of(app, ganger) else {
        return;
    };
    app.world_mut()
        .entity_mut(weapon)
        .insert(ChosenFireMode::new(kind));
}

/// The mode `ganger`'s gun is set to.
pub(crate) fn mode_of(app: &App, ganger: Entity) -> Option<FireModeSpec> {
    let weapon = gun_of(app, ganger)?;
    let modes = app.world().get::<FireMode>(weapon)?;
    chosen_spec(modes, app.world().get::<ChosenFireMode>(weapon))
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

pub(crate) fn select_ganger(app: &mut App, ganger: Entity) -> CellLevel {
    select_ganger_at(app, ganger, SHOOTER_CURSOR_OFFSET)
}

/// Stand `ganger` under the cursor at `offset` and click it, the way the player selects.
pub(crate) fn select_ganger_at(app: &mut App, ganger: Entity, offset: Vec2) -> CellLevel {
    let shooter_cell = hover_at(app, offset);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(shooter_cell, Some(ganger));
    press_left(app);
    app.update();
    clear_mouse(app);
    shooter_cell
}

pub(crate) fn add_probes(app: &mut App) {
    app.add_plugins((
        MessageProbePlugin::<FireRequested>::default(),
        MessageProbePlugin::<MoveRequested>::default(),
        MessageProbePlugin::<SetStanceRequested>::default(),
        MessageProbePlugin::<SetAimingRequested>::default(),
        MessageProbePlugin::<SetFacingRequested>::default(),
        MessageProbePlugin::<ReloadRequested>::default(),
        MessageProbePlugin::<EndTurnRequested>::default(),
        MessageProbePlugin::<ExecuteDownedRequested>::default(),
        MessageProbePlugin::<StabilizeDownedRequested>::default(),
        MessageProbePlugin::<OpenDoorRequested>::default(),
        MessageProbePlugin::<EnterEmplacementRequested>::default(),
        MessageProbePlugin::<ExitEmplacementRequested>::default(),
    ));
}

pub(crate) fn fires(app: &App) -> Vec<FireRequested> {
    probed::<FireRequested>(app)
}

pub(crate) fn active_storey(app: &App) -> i32 {
    app.world()
        .get_resource::<ActiveLevel>()
        .map_or(0, |l| i32::from(***l))
}

//! Shared end-to-end fixture: the fully-armed click-to-shot app (camera /
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
    Accuracy, Aiming, BaseSpread, BattleInProgress, BattleSeed, Cell, CellLevel, DamageProfile,
    DamageType, Direction, Facing, Faction, FatalBias, FightMode, FightModeKind, FightModeSpec,
    FireMode, Handedness, HandlingProfile, HeightBand, Hp, InflictedWounds, Kickback, Level,
    LifeState, Luck, Magazine, MagazineSize, MeleeDamageProfile, MeleeWeaponBundle, OccupancyGrid,
    OccupancyMaintenancePlugin, Position, Reach, ReloadTu, Shooting, Shove, SquadVisibility,
    Stable, Stance, StanceKind, Strikes, Toughness, Tu, TuCost, TuMax, WeaponBundle, WeaponDamage,
    WeaponName, WeaponPunch, WeaponShred, WieldedBy, Wounds,
    acts::SimActsPlugin,
    test_support::{TEST_SEED, insert_sim_resources, single_mode},
};

/// The faction the player controls (matches the litany's gang-0 `PlayerFaction` seed).
pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from the player) — the FIRE target.
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);

/// Synthetic render-target size (physical px) — the `acts.rs` / `picking.rs` harness size.
pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);
/// A cursor offset (from window centre) resolving to a non-origin in-grid SHOOTER cell.
pub(crate) const SHOOTER_CURSOR_OFFSET: Vec2 = Vec2::new(40.0, 32.0);
/// A second, distinct cursor offset resolving to a different in-grid TARGET cell.
pub(crate) const TARGET_CURSOR_OFFSET: Vec2 = Vec2::new(200.0, 160.0);

/// A deterministic `Camera` whose `viewport_to_world_2d` succeeds with no render pipeline
/// (the `picking.rs` synthetic-camera recipe).
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

/// Build the end-to-end headless app: `MinimalPlugins` + the REAL `GdtfBattleInputPlugin`
/// (the click->intent->drain chain) + `SimActsPlugin` (the `FireRequested`->`dispatch_fire`->
/// `fire()` chain) + `OccupancyMaintenancePlugin` (it configures `SimSystems::Simulate`, the
/// set the input plugin's `InputSystems::Gather.before(SimSystems::Simulate)` edge references,
/// so the click drains and the fire dispatch runs in ONE update). Plus the sim resources, a
/// synthetic camera/window so the REAL picker resolves a cursor to a cell, and seeded input
/// buffers.
pub(crate) fn endtoend_app() -> App {
    let mut app = App::new();
    // GTW-322: `update_selection_highlight` (in `GdtfBattleInputPlugin`) spawns its reticle
    // via `Commands::spawn_scene`, which needs an `AssetServer` + the scene schedule. Under
    // `MinimalPlugins` that flush PANICS without `AssetPlugin` + `ScenePlugin` (the
    // spike-documented requirement); adding them keeps the harness asset-inert while the
    // converted spawn path resolves.
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(bevy::scene::ScenePlugin)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    // The canonical sim-resource litany (GTW-576) the `SimActsPlugin` dispatch band
    // reads — the empty grids/ledgers, fog + link graph, the five RNG streams derived
    // from `TEST_SEED` (this suite's historical seed), empty injury content,
    // `CombatTuning::default`, a uniform `FloorCostGrid`, and `PlayerFaction` on gang 0
    // (== `PLAYER_FACTION`). COMPOSED from `gdtf_battle_sim::test_support` instead of
    // mirroring the litany line-by-line; the suite-specific resources follow.
    insert_sim_resources(&mut app, BattleSeed::new(TEST_SEED));
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
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

/// Spawn a FULLY-ARMED, alive, loaded, affordable PLAYER-faction shooter at `cell` facing
/// `facing` — the complete `ShooterQuery` (`With<Weapon>` + weapon stats + ganger state) AND
/// `TargetQuery` (battle surfaces, since the shooter's own liveness reads through the target
/// query) component set. Position matches the occupancy cell so `dispatch_fire`'s arc reads
/// the right actor cell. Ample TU (200) so even an out-of-arc turn-then-fire is affordable.
pub(crate) fn spawn_armed_shooter(app: &mut App, cell: CellLevel, facing: Direction) -> Entity {
    spawn_armed_shooter_inner(app, cell, facing, false)
}

/// As [`spawn_armed_shooter`], but relates a MELEE weapon to the ganger BEFORE the ranged
/// weapon (GTW-505 C5) — so the FIRST entity in the ganger's `Wields` collection is the melee
/// weapon, NOT the gun. The input fire chain must STILL resolve the ranged weapon (via the
/// `MeleeWeapon`-marker filter, not relate order); a regression to the order-dependent
/// `Wields::weapon()` would resolve the magazine-less melee weapon and silently refuse the fire.
pub(crate) fn spawn_armed_shooter_melee_first(
    app: &mut App,
    cell: CellLevel,
    facing: Direction,
) -> Entity {
    spawn_armed_shooter_inner(app, cell, facing, true)
}

/// Spawns the armed PLAYER-faction shooter + relates its ranged weapon; when `melee_first` it
/// ALSO relates a melee weapon BEFORE the ranged one so the melee entity is FIRST in `Wields`.
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
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![single_mode(0.2, 1)]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    // The ganger carries its OWN state only — no weapon stat data (GTW-323 slice 3). The
    // input layer's `can_fire` precheck (`fire_surface::try_fire_request`) now reads the
    // shooter's `Magazine` off the related WEAPON entity (`ganger → Wields → weapon`),
    // not the ganger, mirroring production's `ganger_scene`.
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
    // GTW-505 C5: optionally relate the MELEE weapon FIRST, so the gun is NOT the first entity
    // in `Wields` — proving the input fire chain resolves the ranged weapon by the MeleeWeapon
    // marker filter, not by insertion order.
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
    // GTW-323: the AUTHORITATIVE weapon rides on a related weapon entity (`Wields`), read
    // by the input `can_fire` precheck AND `dispatch_fire`/`fire()` through
    // `ganger → Wields → the weapon entity`. The `WieldedBy` insert hook populates the
    // ganger's `Wields` synchronously in a bare `World` spawn so the end-to-end fire chain
    // resolves the weapon this update (mirroring production's
    // `queue_spawn_related_scenes::<Wields>`).
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    shooter
}

/// Place an ENEMY-faction ganger carrying the full TARGET battle-surface set at `cell` in the
/// occupancy grid (HIGH band, so the march finds a body to strike), returning its entity. Its
/// armor (if any) would live on related piece entities (GTW-323); this bare enemy wears none.
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
    // GTW-11 — mark the enemy cell squad-VISIBLE so the end-to-end fire passes the new
    // targeting-fog rung in `decide_left_click` (without it the fire is refused fail-closed
    // against the harness's empty `SquadVisibility`). The realistic battle state: you fire on a
    // SEEN enemy.
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

/// Set the primary window cursor to a window-centre offset and resolve it via one `update()`
/// (the REAL `pick_hovered_cell`), returning the resolved hovered cell.
pub(crate) fn hover_at(app: &mut App, offset: Vec2) -> CellLevel {
    set_cursor(app, Some(TARGET_SIZE * 0.5 + offset));
    app.update();
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .unwrap_or_else(|| CellLevel::new(Cell::new(0, 0), Level::new(0)))
}

/// Set (or clear) the primary window cursor position (logical px).
pub(crate) fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

//! GTW-289 REGRESSION TEST: clicking an enemy produces a SHOT end-to-end.
//!
//! GTW-289 ("clicking an enemy = no shot") was found ALREADY FIXED on re-test — the original
//! cause was the GTW-297 empty-weapon-registry black-screen, since fixed. This test guards the
//! fix: it wires the full click->shot chain so a future regression that re-silences the fire
//! path fails here.
//!
//! The existing `acts.rs::left_click_emits_one_fire_requested` proves the INPUT half of the
//! chain (a left-click on an enemy emits exactly one `FireRequested`), but stops at the
//! message — its shooter is UNARMED (no `Weapon` marker, no battle surfaces), so `fire()` is a
//! no-op and no shot is observable. This test wires the WHOLE chain:
//!
//! ```text
//!   left-click (real cursor -> InspectTarget)            [gdtf_battle_input]
//!     -> left_click_act decides FIRE                   (decision.rs)
//!       -> ActIntent::Fire pushed                      (apply_left_click)
//!         -> dispatch_act_intents drains -> FireRequested emitted   (intent/seam.rs)
//!           -> dispatch_fire gates (arc) + runs fire() (acts/fire.rs, SimActsPlugin)
//!             -> observable: shooter TU debited, magazine decremented  (fire/volley.rs)
//! ```
//!
//! It spawns a FULLY-ARMED shooter (the `ShooterQuery` `With<Weapon>` set + the `TargetQuery`
//! battle surfaces the shooter's own liveness reads through) and a battle-surface ENEMY in the
//! occupancy grid, places them in a LEGITIMATE firing situation (in range, in-arc, alive,
//! loaded, affordable), synthesizes the click, runs one `update()`, and asserts the shot ran by
//! the SAME seed-agnostic relation the sim's own AC3 test uses: the shooter's TU strictly
//! dropped (the up-front mode charge) — hit/miss is seed-dependent, the CHARGE is not.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out (a)); no
//! function here takes `&mut World`/`&World`.

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
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedShooter};
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
use gdtf_test_utils::{clear_mouse, press_left};

/// The faction the player controls (matches the litany's gang-0 `PlayerFaction` seed).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from the player) — the FIRE target.
const ENEMY_FACTION: Faction = Faction::new(1);

/// Synthetic render-target size (physical px) — the `acts.rs` / `picking.rs` harness size.
const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);
/// A cursor offset (from window centre) resolving to a non-origin in-grid SHOOTER cell.
const SHOOTER_CURSOR_OFFSET: Vec2 = Vec2::new(40.0, 32.0);
/// A second, distinct cursor offset resolving to a different in-grid TARGET cell.
const TARGET_CURSOR_OFFSET: Vec2 = Vec2::new(200.0, 160.0);

/// A deterministic `Camera` whose `viewport_to_world_2d` succeeds with no render pipeline
/// (the `picking.rs` synthetic-camera recipe).
fn synthetic_camera() -> Camera {
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
fn endtoend_app() -> App {
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
fn spawn_armed_shooter(app: &mut App, cell: CellLevel, facing: Direction) -> Entity {
    spawn_armed_shooter_inner(app, cell, facing, false)
}

/// As [`spawn_armed_shooter`], but relates a MELEE weapon to the ganger BEFORE the ranged
/// weapon (GTW-505 C5) — so the FIRST entity in the ganger's `Wields` collection is the melee
/// weapon, NOT the gun. The input fire chain must STILL resolve the ranged weapon (via the
/// `MeleeWeapon`-marker filter, not relate order); a regression to the order-dependent
/// `Wields::weapon()` would resolve the magazine-less melee weapon and silently refuse the fire.
fn spawn_armed_shooter_melee_first(app: &mut App, cell: CellLevel, facing: Direction) -> Entity {
    spawn_armed_shooter_inner(app, cell, facing, true)
}

/// Spawns the armed PLAYER-faction shooter + relates its ranged weapon; when `melee_first` it
/// ALSO relates a melee weapon BEFORE the ranged one so the melee entity is FIRST in `Wields`.
fn spawn_armed_shooter_inner(
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
fn place_armed_enemy(app: &mut App, cell: CellLevel) -> Entity {
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
fn hover_at(app: &mut App, offset: Vec2) -> CellLevel {
    set_cursor(app, Some(TARGET_SIZE * 0.5 + offset));
    app.update();
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .unwrap_or_else(|| CellLevel::new(Cell::new(0, 0), Level::new(0)))
}

/// Set (or clear) the primary window cursor position (logical px).
fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// GTW-289 — a left-click on an ENEMY in a LEGITIMATE firing situation produces a SHOT
/// end-to-end: the shooter's TU strictly drops (the up-front mode charge) after the click.
#[test]
fn click_on_enemy_produces_a_shot_endtoend() {
    let mut app = endtoend_app();

    // Resolve the shooter cell + target cell from the real picker first, so the shooter's
    // Position / occupancy cell and the enemy's occupancy cell are the SAME cells the click
    // will resolve.
    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    // Place the shooter facing TOWARD the target (in-arc) so the simplest fire path runs;
    // ample TU also covers the out-of-arc turn-then-fire branch if the facing is off.
    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    let shooter = spawn_armed_shooter(&mut app, shooter_cell, facing);
    // Register the shooter in the occupancy grid at its own cell (so the SELECT click finds it)
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(shooter_cell, Some(shooter));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    }
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    // SELECT the shooter: cursor over its cell, fresh left-click, one update.
    let _ = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    press_left(&mut app);
    app.update();
    clear_mouse(&mut app);
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "the player-faction shooter must be SELECTED before firing",
    );

    // Move the cursor onto the ENEMY cell and snapshot the shooter's TU before the fire click.
    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    assert!(tu_before.is_some(), "shooter has a Tu pool");

    // FIRE: a fresh left-click on the enemy cell. One update runs the WHOLE chain
    // (click -> intent -> drain -> FireRequested -> dispatch_fire -> fire()).
    press_left(&mut app);
    app.update();

    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    let shot_ran = matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b);
    assert!(
        shot_ran,
        "clicking an enemy in a legitimate firing situation must run fire() (the shooter's \
         TU must drop by the mode charge) — tu {tu_before:?} -> {tu_after:?}. If TU is \
         UNCHANGED the fire path is silent end-to-end (GTW-289).",
    );

    // The selection must be UNCHANGED by a FIRE edge (GTW-238 — FIRE does not re-select).
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "a FIRE edge leaves the selection untouched",
    );
}

/// GTW-505 C5 — the INPUT-LAYER zero-ranged-regression proof, ORDERING-INDEPENDENT: a shooter
/// wielding BOTH a melee weapon (related FIRST) AND a ranged weapon still fires the RANGED
/// weapon end-to-end. The input `can_fire` precheck (`fire_surface::try_fire_request`) resolves
/// the gun via `Wields::ranged_weapon` (the `MeleeWeapon`-marker filter), NOT `Wields::weapon`
/// (the FIRST related entity) — so the melee weapon being first never silences the fire.
///
/// PIN: with the OLD order-dependent `Wields::weapon()`, the FIRST related entity here is the
/// magazine-less melee weapon; `try_fire_request`'s `(Magazine, Handedness)` read would miss
/// it and fail closed → NO `FireRequested`, the shooter's TU UNCHANGED. Pinning the TU drop
/// (the mode charge) AND the RANGED magazine decrement proves the gun was resolved despite the
/// melee weapon sitting first in `Wields`.
#[test]
fn click_on_enemy_fires_the_ranged_weapon_even_with_a_melee_weapon_related_first() {
    let mut app = endtoend_app();

    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    // The MELEE weapon is related FIRST (so it is first in `Wields`), the ranged gun second.
    let shooter = spawn_armed_shooter_melee_first(&mut app, shooter_cell, facing);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(shooter_cell, Some(shooter));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    }
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    // The ranged weapon's magazine BEFORE firing — resolved the ranged way (excluding the melee
    // weapon) so this reads the GUN's count, never the magazine-less melee entity.
    let rounds_before = ranged_magazine_rounds(&app, shooter);
    assert_eq!(
        rounds_before,
        Some(10),
        "precondition: the RANGED weapon (resolved excluding the melee one) holds 10 rounds",
    );

    // SELECT the shooter.
    let _ = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    press_left(&mut app);
    app.update();
    clear_mouse(&mut app);
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "the player-faction shooter must be SELECTED before firing",
    );

    // FIRE on the enemy.
    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    press_left(&mut app);
    app.update();

    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    let shot_ran = matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b);
    assert!(
        shot_ran,
        "with a melee weapon related FIRST, the input fire chain must STILL fire the ranged \
         weapon (the shooter's TU must drop) — tu {tu_before:?} -> {tu_after:?}. If TU is \
         UNCHANGED the input `can_fire` resolved the magazine-less melee weapon (the \
         order-dependent `Wields::weapon()` regression GTW-505 C5 guards).",
    );

    // The RANGED magazine decremented — proof the GUN was the entity `fire()` resolved + spent,
    // not the magazine-less melee weapon (a misresolution leaves rounds untouched).
    let rounds_after = ranged_magazine_rounds(&app, shooter);
    assert!(
        matches!((rounds_before, rounds_after), (Some(b), Some(a)) if a < b),
        "the RANGED magazine must drop (the burst spent it) — proof the gun, not the \
         magazine-less melee weapon, was fired: {rounds_before:?} -> {rounds_after:?}",
    );
}

/// The current round count of the shooter's RANGED weapon magazine, resolved the ranged way
/// (`Wields::ranged_weapon`, excluding the `MeleeWeapon`-marked entity) so it reads the GUN's
/// magazine even when a melee weapon is related first. `None` when unarmed / no ranged weapon.
fn ranged_magazine_rounds(app: &App, shooter: Entity) -> Option<u16> {
    use gdtf_battle_sim::{MeleeWeapon, Wields};
    let melee_entities: bevy::platform::collections::HashSet<Entity> = {
        let mut q = app
            .world()
            .try_query_filtered::<Entity, With<MeleeWeapon>>()?;
        q.iter(app.world()).collect()
    };
    app.world()
        .get::<Wields>(shooter)
        .and_then(|w| w.ranged_weapon(|e| melee_entities.contains(&e)))
        .and_then(|ranged| app.world().get::<Magazine>(ranged))
        .map(|m| *m.rounds())
}

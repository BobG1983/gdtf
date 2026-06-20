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
use gdtf_battle_presenter::{ActiveLevel, WorldCamera};
use gdtf_battle_sim::{
    Accuracy, Aiming, BaseSpread, BattleInProgress, BattleSeed, Cell, CellLevel, CoverLedger,
    DamageProfile, DamageType, Direction, Facing, Faction, FatalBias, FireMode, FireModeSpec,
    HandlingProfile, HeightBand, Hp, InflictedWounds, Kickback, Level, LifeState, Luck, Magazine,
    MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, OccupancyGrid,
    OccupancyMaintenancePlugin, PlayerFaction, Position, ReloadTu, Shooting, SimRng, Stable,
    Stance, StanceKind, SurfaceGrid, Toughness, Tu, TuMax, WeaponBundle, WeaponDamage, WeaponName,
    WeaponPunch, WeaponShred, Wounds, acts::SimActsPlugin, tuning::CombatTuning,
};

/// The faction the player controls (matches `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from the player) — the FIRE target.
const ENEMY_FACTION: Faction = Faction::new(1);

/// Synthetic render-target size (physical px) — the `acts.rs` / `picking.rs` harness size.
const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);
/// A cursor offset (from window centre) resolving to a non-origin in-grid SHOOTER cell.
const SHOOTER_CURSOR_OFFSET: Vec2 = Vec2::new(40.0, 32.0);
/// A second, distinct cursor offset resolving to a different in-grid TARGET cell.
const TARGET_CURSOR_OFFSET: Vec2 = Vec2::new(200.0, 160.0);

/// One single-shot fire-mode spec (arbitrary, non-pinned per-mode numbers).
const fn single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

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
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    app.world_mut().insert_resource(ActiveLevel(Level::new(0)));
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(SimRng::from_seed(BattleSeed::new(0x5A1C_AC75)));
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
            FireMode::new(vec![single_mode()]),
            Stable::new(true),
        ),
    );
    app.world_mut()
        .spawn((
            bundle,
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
                gdtf_battle_sim_worn_suit(),
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id()
}

/// A worn suit (thin, so a hit is likely to land an effect) built from the sim's armor
/// constructors. Kept tiny to mirror the sim's `worn_suit(0, 0, 1, 0)` fixture without
/// re-exporting it here.
const fn gdtf_battle_sim_worn_suit() -> gdtf_battle_sim::WornArmor {
    use gdtf_battle_sim::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec,
        ArmorType, WornArmor,
    };
    WornArmor::seed_from(&ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(0),
        ArmorIntegrity::new(1),
        ArmorHardness::new(0),
        ArmorType::DEFAULT,
    )))
}

/// Place an ENEMY-faction ganger carrying the full TARGET battle-surface set at `cell` in the
/// occupancy grid (HIGH band, so the march finds a body to strike), returning its entity.
fn place_armed_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app
        .world_mut()
        .spawn((
            ENEMY_FACTION,
            Hp::new(30),
            Wounds::new(6),
            LifeState::Alive,
            gdtf_battle_sim_worn_suit(),
            InflictedWounds::default(),
            Toughness::new(1.0),
            Luck::new(0.0),
        ))
        .id();
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(cell, Some(enemy));
        grid.set_occupant_band(cell, Some(HeightBand::High));
    }
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

/// Press the just-pressed edge of the left mouse button.
fn press_left(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
}

/// Release + clear the mouse edges so a later press is a fresh just-pressed.
fn clear_mouse(app: &mut App) {
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release(MouseButton::Left);
    mouse.clear();
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
    let facing = Direction::from_cells(
        Cell::new(shooter_cell.x, shooter_cell.y),
        Cell::new(target_cell.x, target_cell.y),
    )
    .unwrap_or(Direction::East);
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

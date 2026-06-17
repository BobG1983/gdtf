//! GTW-250 (AC5): the KEYBOARD pan path through the REAL registered `pan_camera`
//! system, in a live battle.
//!
//! Drives the full GDTF state machine into a live `GameState::BattleScape` (a valid
//! two-ganger situation, so `setup_battle` inserts the sim's `BattleInProgress` +
//! `PlayerFaction` — the battle gate the camera systems run under) and then exercises the
//! presenter's `pan_camera` exactly as the app wires it (in `Update`, battle-gated,
//! `.before(clamp_camera_to_bounds)`). Pressing `KeyCode::KeyW` (set via the
//! `ButtonInput<KeyCode>` resource, which `InputPlugin` provides under `register_headless`)
//! moves the `WorldCamera` `translation.y` UP and the clamp keeps it inside the battlefield;
//! pressing nothing leaves the camera put.
//!
//! The mouse-edge + gamepad paths are covered by their pure helper unit tests in
//! `gdtf_battle_presenter::world_camera` + in-engine QA (real window cursor / real gamepad
//! axes are TBD-Bevy-harness per `verification.md`).
//!
//! All `app.world_mut()` / camera mutation is in the TEST BODY — the accepted headless
//! idiom (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.
//! The camera is repositioned mid-battlefield in the test body purely to give the pan room
//! to move (so the clamp is not the boundary case) — the SYSTEM under test is the real
//! registered `pan_camera`, not a copy.

use bevy::{
    ecs::prelude::With,
    input::{ButtonInput, keyboard::KeyCode},
    math::Vec2,
    state::state::{NextState, State},
    transform::components::Transform,
};
use gdtf_app::test_support::{BattleScapeState, GameState, LoadedSituation, RunningState};
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        SourceArmor,
    },
    battle::{BattleInProgress, PlayerFaction},
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Shooting, Stance, StanceKind,
        Toughness, Tu, Wounds,
    },
    metric::{Cell, CellLevel, Level},
    situation::{GangerSpawn, Situation},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
        MagazineSize, ModeConeMult, ModeName, ModeShots, ModeTuPercent, Stable, WeaponDamage,
        WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// The weapon KEY every fixture ganger references — present in [`weapon_registry`].
const TEST_WEAPON_KEY: &str = "test-weapon";

/// A registry holding the one [`TEST_WEAPON_KEY`] weapon the fixture gangers
/// reference, standing in for the `Load`-built registry (GTW-257) so the deep-walk
/// setup arms each ganger.
fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        WeaponSpec {
            base_spread:   BaseSpread::new(0.25),
            accuracy:      Accuracy::new(1.0),
            kickback:      Kickback::new(0.4),
            fatal_bias:    FatalBias::new(7.0),
            damage:        WeaponDamage::new(12),
            punch:         WeaponPunch::new(5),
            shred:         WeaponShred::new(3),
            damage_type:   DamageType::Kinetic,
            magazine_size: MagazineSize::new(30),
            fire_mode:     FireMode::Single {
                single: FireModeSpec::new(
                    ModeName::new("single".to_owned()),
                    ModeConeMult::new(1.0),
                    ModeTuPercent::new(0.5),
                    ModeShots::new(1),
                ),
            },
            stable:        Stable::new(false),
        },
    )])
}

/// A budget large enough to drive the deep walk into the live battle, but bounded so a
/// machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// How many updates to hold a pan key — enough for the per-update virtual-time delta
/// (`FixedTimesteps(1)`, ~1/64 s) to accumulate a clearly measurable, non-clamped move.
const PAN_UPDATES: u32 = 12;

/// Build a `(cell, level)` key from raw coordinates.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// An arbitrary roster armor record (distinct per-part magnitudes, NOT shipped tuning).
const fn arbitrary_armor(base: i32) -> SourceArmor {
    SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Build an authored ganger at `at` with arbitrary-but-valid component values.
fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(StanceKind::Crouching),
        aiming: Aiming::new(true),
        hp: Hp::new(40),
        wounds: Wounds::new(3),
        tu: Tu::new(60),
        life_state: LifeState::Alive,
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        armor: arbitrary_armor(i32::from(faction) + 1),
        // Every fixture ganger references the one TEST_WEAPON_KEY in weapon_registry.
        weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
    }
}

/// A valid two-ganger fixture situation (link-free, so it validates trivially) — enough for
/// `setup_battle` to succeed and insert `BattleInProgress` + `PlayerFaction`.
fn two_ganger_situation() -> Situation {
    Situation {
        gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)],
        ..Situation::new()
    }
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app with the persistent `Load` resources + a two-ganger
/// situation fixture (so the live battle's `setup_battle` succeeds).
fn walk_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // The Load-built WeaponRegistry (GTW-257) so the live battle's setup arms gangers.
    app.world_mut().insert_resource(weapon_registry());
    app.world_mut()
        .insert_resource(LoadedSituation(two_ganger_situation()));
    app
}

/// Stands in for the player at the menu (it no longer auto-advances, GTW-121): advances
/// until [`RunningState::Menu`] rests, then queues `Menu → Options`.
fn drive_past_menu(app: &mut bevy::app::App) -> bool {
    let reached = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if reached {
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached
}

/// Drives into a LIVE battle: down to `GameState::BattleScape` AND past `Generation` (so the
/// successful `setup_battle` has inserted `BattleInProgress` + `PlayerFaction`, the gate the
/// camera systems run under). Returns whether the live battle was reached within budget.
fn drive_into_live_battle(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    let reached_scape = advance_until(
        app,
        |app| game_state(app) == Some(GameState::BattleScape),
        BUDGET,
    );
    if !reached_scape {
        return false;
    }
    // Past Generation: the setup Ok path (which inserts BattleInProgress + PlayerFaction)
    // has run, so the camera gate is satisfied.
    advance_until(
        app,
        |app| {
            app.world().get_resource::<BattleInProgress>().is_some()
                && app.world().get_resource::<PlayerFaction>().is_some()
        },
        BUDGET,
    )
}

/// Reads the single `WorldCamera`'s translation `xy`.
fn camera_xy(app: &mut bevy::app::App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

/// Sets the single `WorldCamera`'s translation `xy` (a TEST-BODY world mutation), giving the
/// pan room to move so the clamp is not the boundary case.
fn set_camera_xy(app: &mut bevy::app::App, to: Vec2) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&mut Transform, With<WorldCamera>>();
    for mut transform in query.iter_mut(world) {
        transform.translation.x = to.x;
        transform.translation.y = to.y;
    }
}

/// The battlefield ground-plane world bounds `(min, max)` — the four corner cells of the
/// `GRID_WIDTH x GRID_HEIGHT` ground extent through `cell_to_world` (the SAME projection the
/// clamp uses). Kept local so the assertions check a RELATION, not a pinned magnitude.
fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        gdtf_battle_presenter::cell_to_world(Cell::new(0, 0), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(w, 0), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(0, h), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(w, h), Level::new(0)),
    ];
    let mut min = Vec2::new(corners[0].x, corners[0].y);
    let mut max = min;
    for c in corners {
        let p = Vec2::new(c.x, c.y);
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

/// The battlefield centre (world space) — a position with pan headroom on every axis.
fn battlefield_centre() -> Vec2 {
    let (min, max) = battlefield_bounds();
    (min + max) * 0.5
}

/// Holds `key` pressed across `PAN_UPDATES` updates so the per-frame pan accumulates a
/// measurable move; the `ButtonInput<KeyCode>` is re-pressed each frame because
/// `InputPlugin`'s `clear` runs every update.
fn hold_key_for_pan(app: &mut bevy::app::App, key_code: KeyCode) {
    for _ in 0..PAN_UPDATES {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key_code);
        app.update();
    }
}

/// AC5 — pressing `KeyCode::KeyW` in a live battle pans the `WorldCamera` UP (`+Y`) through
/// the REAL registered `pan_camera`, and the clamp (which runs after) keeps it inside the
/// battlefield; pressing nothing leaves the camera put.
#[test]
fn keyboard_w_pans_camera_up_within_bounds() {
    let mut app = walk_app();
    assert!(
        drive_into_live_battle(&mut app),
        "the walk should reach a LIVE battle (BattleInProgress + PlayerFaction) within {BUDGET} \
         updates; last observed GameState was {:?}, BattleScapeState {:?}",
        game_state(&app),
        battlescape_state(&app),
    );

    // The system must be battle-gated AND actually running — both witnesses present.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some()
            && app.world().get_resource::<PlayerFaction>().is_some(),
        "precondition: the live battle's BattleInProgress + PlayerFaction gate the camera systems",
    );

    // Park the camera at the battlefield centre so the pan has room in every direction (the
    // clamp is not the boundary case for a small up-pan).
    let centre = battlefield_centre();
    set_camera_xy(&mut app, centre);
    // Settle the clamp once so the baseline is a clamped, stable position.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let baseline = camera_xy(&mut app);

    // --- Control: no key pressed → the camera does not move. ---
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let no_press = camera_xy(&mut app);
    assert_eq!(
        no_press, baseline,
        "with no pan key pressed, the camera must NOT move",
    );

    // --- W held → the camera pans UP (+Y). ---
    hold_key_for_pan(&mut app, KeyCode::KeyW);
    let after_w = camera_xy(&mut app);
    assert!(
        after_w.y > baseline.y,
        "holding W must pan the WorldCamera UP (+Y): baseline y {} -> after {}",
        baseline.y,
        after_w.y,
    );

    // The pan stayed within the battlefield bounds — the clamp ran AFTER the pan (it is the
    // last writer). The half-viewport is the headless default unit rect, so the camera centre
    // must sit within [min, max] (a far tighter bound than [min+half, max-half]).
    let (min, max) = battlefield_bounds();
    assert!(
        after_w.y <= max.y + f32::EPSILON && after_w.y >= min.y - f32::EPSILON,
        "after the pan the camera y ({}) must stay within the battlefield y bounds [{}, {}] — the \
         clamp ran after the pan",
        after_w.y,
        min.y,
        max.y,
    );
    assert!(
        after_w.x <= max.x + f32::EPSILON && after_w.x >= min.x - f32::EPSILON,
        "the camera x must stay within the battlefield x bounds",
    );
}

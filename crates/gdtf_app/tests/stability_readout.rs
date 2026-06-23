//! GTW-345 — the battlescape status-panel STABILITY READOUT, driven through the REAL app
//! stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine down
//! to `BattleScapeState::BattleRunning`, where the real status-panel plugin spawns the
//! stability-readout bar and its update system repaints it under the `BattleInProgress` gate.
//! The bar is a HUD `Node` widget — there is no shader to render-readback, so the Node/fill
//! component-state assert IS the binding proof (an in-engine VISUAL screenshot is owed-but-TBD
//! because the dev capture is broken; see the C9 note in the ticket).
//!
//! They cover:
//!
//! - **C6(1)** — an armed, selected shooter over a known faced cover → the bar's fill is the
//!   value derived from the authoritative `stability_for` `ConeMult` for that shooter (computed
//!   DIRECTLY from the sim surface in the test body and compared to the DISPLAYED fill, so a
//!   wrong source/value diverges).
//! - **C6(2)** — no selection → the bar shows the EMPTY (zero) state.
//! - **C6(3)** — mutating the shooter's Stance and updating → the bar fill CHANGES (it tracks
//!   the live shooter state).

use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::Val};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState, StabilityBar};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Aiming, ArmorHardness, ArmorProtection, Cell, CellLevel, ConeMult, CoverEntry, CoverHp,
    CoverLedger, Direction, Facing, Faction, FireMode, FireModeSpec, HeightBand, Level, LifeState,
    Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Position, ReloadTu,
    Shooter, Stable, Stance, StanceKind, Tu, TuMax, WeaponBundle, WieldedBy, faced_cell,
    stability_for,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, HandlingProfile, Kickback,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{ProgressBarFill, theme::default_theme};

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine that
/// never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// Reads the current [`BattleScapeState`] if active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives the real stack to `BattleScapeState::BattleRunning`, where the status panel is live.
/// (The `weapon_panel.rs` harness precedent — a ganger-free default battle still reaches
/// `BattleRunning` with `BattleInProgress` present, and the test spawns its own shooter.)
fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        at_battle,
        "the walk should reach BattleScapeState::BattleRunning; last was {:?}",
        battlescape_state(&app),
    );
    app
}

/// The single entity carrying marker `M`, or `None` if not exactly one.
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The fill PERCENT of the `ProgressBar` rooted at `track` — reads the `ProgressBarFill` child's
/// `Node.width`. `None` if missing.
fn bar_fill_at(app: &App, track: Entity) -> Option<f32> {
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(track)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for kid in kids {
        if app.world().get::<ProgressBarFill>(kid).is_some()
            && let Some(node) = app.world().get::<Node>(kid)
            && let Val::Percent(p) = node.width
        {
            return Some(p);
        }
    }
    None
}

/// The fill PERCENT of the status-panel stability bar.
fn stability_fill(app: &mut App) -> Option<f32> {
    let track = single_with::<StabilityBar>(app)?;
    bar_fill_at(app, track)
}

/// The DISPLAY normalization the readout uses, reproduced here so the test compares the
/// displayed fill against a value derived from the live sim surface (the documented contract:
/// lower cone = steadier = fuller bar; steadiness = the narrowing complement `1 - cone_mult`,
/// clamped, as a PERCENT).
fn expected_fill_percent(cone_mult: ConeMult) -> f32 {
    (1.0 - *cone_mult).clamp(0.0, 1.0) * 100.0
}

/// A shooter's grid placement + posture, grouped so the spawn helper stays under the arg-count
/// gate.
struct ShooterPlacement {
    cell:   Cell,
    facing: Direction,
    stance: StanceKind,
    aiming: bool,
}

/// A weapon kit (arbitrary magnitudes — not shipped tuning), with the `stable` tag exposed so a
/// test can pin a known stability contribution.
fn weapon_kit(stable: bool) -> WeaponBundle {
    WeaponBundle::new(
        WeaponName::new("Autogun".to_owned()),
        BaseSpread::new(0.2),
        Accuracy::new(1.0),
        Kickback::new(0.1),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(10),
            WeaponPunch::new(2),
            WeaponShred::new(1),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(20, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.3),
                ModeShots::new(1),
            )]),
            Stable::new(stable),
        ),
    )
}

/// Spawns an armed ganger (the full `Shooter` view: Stance/Aiming/Position/Facing) wielding a
/// weapon on a related WEAPON entity (`WieldedBy` → the `Wields` insert hook), SELECTS it, and
/// returns its entity. (The `weapon_panel.rs` `spawn_armed_and_select` precedent.)
fn spawn_armed_and_select(app: &mut App, place: ShooterPlacement) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(place.cell, Level::new(0))),
            Faction::new(0),
            Facing::new(place.facing),
            Stance::new(place.stance),
            Aiming::new(place.aiming),
            Tu::new(100),
            TuMax::new(100),
            LifeState::Alive,
        ))
        .id();
    app.world_mut()
        .spawn((WieldedBy(ganger), weapon_kit(false)));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

/// A HIGH-band cover entry — a tall wall, the brace-engaging band the stability tests use.
const fn high_cover() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(100),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    )
}

/// Inserts a [`CoverLedger`] with a HIGH cover at the cell the shooter at `cell`/`facing` faces
/// (the same faced cell `stability_for` reads internally), so the brace gate has something to
/// engage.
fn seed_faced_cover(app: &mut App, cell: Cell, facing: Direction) {
    let mut ledger = CoverLedger::new();
    let (faced_cell_xy, faced_level) = faced_cell(
        &Position::new(CellLevel::new(cell, Level::new(0))),
        &Facing::new(facing),
    );
    ledger.insert(CellLevel::new(faced_cell_xy, faced_level), high_cover());
    app.world_mut().insert_resource(ledger);
}

// ---------------------------------------------------------------------------------
// C6(1) — the bar carries the stability_for-derived ConeMult for the selected shooter.
// ---------------------------------------------------------------------------------

#[test]
fn stability_readout_shows_the_stability_for_value() {
    let mut app = battle_running_app();
    let cell = Cell::new(3, 3);
    let facing = Direction::East;
    seed_faced_cover(&mut app, cell, facing);
    spawn_armed_and_select(
        &mut app,
        ShooterPlacement {
            cell,
            facing,
            stance: StanceKind::Standing,
            aiming: false,
        },
    );
    app.update();

    // Compute the expected ConeMult DIRECTLY from the live sim surface (the same inputs the
    // system assembles): the shooter view, the weapon's `stable` tag (false here), the model
    // cover ledger, and the tuning.
    let stance = Stance::new(StanceKind::Standing);
    let aiming = Aiming::new(false);
    let position = Position::new(CellLevel::new(cell, Level::new(0)));
    let facing_c = Facing::new(facing);
    let shooter = Shooter {
        stance:   &stance,
        aiming:   &aiming,
        position: &position,
        facing:   &facing_c,
    };
    let ledger = {
        let mut l = CoverLedger::new();
        let (fc, fl) = faced_cell(&position, &facing_c);
        l.insert(CellLevel::new(fc, fl), high_cover());
        l
    };
    let tuning = CombatTuning::default();
    let (cone_mult, _recoil) = stability_for(&shooter, Stable::new(false), &ledger, &tuning);
    let expected = expected_fill_percent(cone_mult);

    let fill = stability_fill(&mut app).unwrap_or(-1.0);
    assert!(
        (fill - expected).abs() < 0.5,
        "stability bar fill must be the stability_for-derived steadiness ({expected}%), got \
         {fill}%",
    );
    // The cover braces a standing shooter (HIGH band) → a steadier-than-baseline cone → the bar
    // must read non-empty (a wrong source would read empty / wrong).
    assert!(
        fill > 0.5,
        "a braced shooter must read a non-empty bar, got {fill}%"
    );
}

// ---------------------------------------------------------------------------------
// C6(2) — no selection → the EMPTY (zero) state.
// ---------------------------------------------------------------------------------

#[test]
fn stability_readout_empty_with_no_selection() {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();

    let fill = stability_fill(&mut app).unwrap_or(-1.0);
    assert!(
        fill.abs() < 0.5,
        "with no selection the stability bar must show the empty (zero) state, got {fill}%",
    );
}

// ---------------------------------------------------------------------------------
// C6(3) — mutating the shooter's Stance changes the readout.
// ---------------------------------------------------------------------------------

#[test]
fn stability_readout_tracks_stance_change() {
    let mut app = battle_running_app();
    let cell = Cell::new(3, 3);
    let facing = Direction::East;
    // No faced cover this time, so the brace gate's per-stance band requirement is what moves
    // the readout: a standing vs prone shooter has a different stance stability contribution,
    // so the cone_mult — and the bar fill — differ.
    let ganger = spawn_armed_and_select(
        &mut app,
        ShooterPlacement {
            cell,
            facing,
            stance: StanceKind::Standing,
            aiming: false,
        },
    );
    app.update();
    let standing = stability_fill(&mut app).unwrap_or(-1.0);

    // Mutate the shooter's stance to Prone (the steadiest posture) and update.
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();
    let prone = stability_fill(&mut app).unwrap_or(-1.0);

    assert!(
        (standing - prone).abs() > 0.5,
        "changing the shooter's stance must change the stability readout: standing {standing}% \
         vs prone {prone}%",
    );
}

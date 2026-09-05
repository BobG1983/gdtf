//! Stability bar: mirrors `stability_for`; empty with no selection; tracks stance.
use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::Val};
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    aim::{Shooter, stability_for},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    faced_cell::faced_cell,
    ganger::{Aiming, Facing, TuMax},
    injuries::InjuryRegistry,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, Position, Stance, StanceKind, Tu,
    },
    stability::{ConeMult, StabilityTerms},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy,
    },
};
use gdtf_game::test_support::{AppState, BattleScapeState, RunningState, StabilityBar};
use gdtf_ui::{ProgressBarFill, theme::default_theme};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battle_running_app() -> App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    app
}

fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

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

fn stability_fill(app: &mut App) -> Option<f32> {
    let track = single_with::<StabilityBar>(app)?;
    bar_fill_at(app, track)
}

fn expected_fill_percent(cone_mult: ConeMult) -> f32 {
    (1.0 - *cone_mult).clamp(0.0, 1.0) * 100.0
}

struct ShooterPlacement {
    cell:   Cell,
    facing: Direction,
    stance: StanceKind,
    aiming: bool,
}

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
            Magazine::new(
                LoadedRounds::new(20),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.3),
                ModeShots::new(1),
            )]),
            Stable::new(stable),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

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
        .spawn((WieldedBy::new(ganger), weapon_kit(false)));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

const fn high_cover() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(100),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    )
}

fn seed_faced_cover(app: &mut App, cell: Cell, facing: Direction) {
    let mut ledger = CoverLedger::new();
    let (faced_cell_xy, faced_level) = faced_cell(
        &Position::new(CellLevel::new(cell, Level::new(0))),
        &Facing::new(facing),
    );
    ledger.insert(CellLevel::new(faced_cell_xy, faced_level), high_cover());
    app.world_mut().insert_resource(ledger);
}

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

    let stance = Stance::new(StanceKind::Standing);
    let aiming = Aiming::new(false);
    let position = Position::new(CellLevel::new(cell, Level::new(0)));
    let facing_c = Facing::new(facing);
    let shooter = Shooter {
        stance:     &stance,
        aiming:     &aiming,
        position:   &position,
        facing:     &facing_c,
        suppressed: None,
    };
    let ledger = {
        let mut l = CoverLedger::new();
        let (fc, fl) = faced_cell(&position, &facing_c);
        l.insert(CellLevel::new(fc, fl), high_cover());
        l
    };
    let tuning = CombatTuning::default();
    let (cone_mult, _recoil) = stability_for(&shooter, StabilityTerms::default(), &ledger, &tuning);
    let expected = expected_fill_percent(cone_mult);

    let fill = stability_fill(&mut app).unwrap_or(-1.0);
    assert!(
        (fill - expected).abs() < 0.5,
        "stability bar fill must be the stability_for-derived steadiness ({expected}%), got \
         {fill}%",
    );
    assert!(
        fill > 0.5,
        "a braced shooter must read a non-empty bar, got {fill}%"
    );
}

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

#[test]
fn stability_readout_tracks_stance_change() {
    let mut app = battle_running_app();
    let cell = Cell::new(3, 3);
    let facing = Direction::East;
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

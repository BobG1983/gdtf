//! Blast resolve: no phantom miss lines when a grenade impact resolves.
use bevy::prelude::*;
use gdtf_app::test_support::{AppState, BattleScapeState, CombatLogLine, RunningState};
use gdtf_battle_presenter::ShotImpactResolved;
use gdtf_battle_sim::{
    acts::ThrowGrenadeRequested,
    ganger::{Luck, Tu},
    magazine::{Magazine, ReloadTu},
    prelude::{Cell, CellLevel, Level, Position},
    test_support::test_weapon_spec,
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, FireModeSpec, HitType,
        Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, TrajectoryStyle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponSpec, WieldedBy,
    },
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, MessageProbe, MessageProbePlugin, advance_until};

const BUDGET: u32 = 512;

const CHAIN_BUDGET: u32 = 64;

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn battle_running_app() -> Option<App> {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    if !advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    ) {
        return None;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);

    if !advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    ) {
        return None;
    }
    Some(app)
}

fn grenade_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        accuracy: Accuracy::new(0.8),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(2.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Blast,
        magazine: Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        fire_mode: FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.35),
            ModeShots::new(1),
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
        )]),
        trajectory: TrajectoryStyle::Arc,
        ..test_weapon_spec()
    }
}

fn spawn_armed_thrower(app: &mut App, at: CellLevel) -> Entity {
    let thrower = app
        .world_mut()
        .spawn((Position::new(at), Luck::new(10.0), Tu::new(250)))
        .id();
    let (bundle, _siblings) =
        grenade_spec().into_bundle(WeaponName::new("gtw559-test-grenade".to_owned()));
    app.world_mut().spawn((bundle, WieldedBy::new(thrower)));
    thrower
}

fn add_impact_probe(app: &mut App) {
    app.add_plugins(MessageProbePlugin::<ShotImpactResolved>::default());
}

fn blast_signal_seen(app: &App) -> bool {
    app.world()
        .get_resource::<MessageProbe<ShotImpactResolved>>()
        .is_some_and(|probe| {
            probe
                .seen()
                .iter()
                .any(|impact| impact.report.is_none() && impact.shooter == Entity::PLACEHOLDER)
        })
}

fn log_line_texts(app: &mut App) -> Vec<String> {
    let entities: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<CombatLogLine>>();
        q.iter(app.world()).collect()
    };
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.as_str().to_owned()))
        .collect()
}

#[test]
fn a_blast_detonation_renders_no_phantom_someone_missed_line() {
    let app_opt = battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real Load + descent must reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };
    add_impact_probe(&mut app);

    let thrower_at = CellLevel::new(Cell::new(30, 30), Level::new(0));
    let target_at = CellLevel::new(Cell::new(32, 30), Level::new(0));
    let thrower = spawn_armed_thrower(&mut app, thrower_at);
    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower, target_at));

    let detonated = advance_until(&mut app, blast_signal_seen, CHAIN_BUDGET);
    assert!(
        detonated,
        "the real detonation must ride the presenter impact pipeline (ThrowResolved -> \
         PendingImpact::for_blast -> ShotImpactResolved with a placeholder shooter + None \
         report) within the chain budget",
    );

    for _ in 0..4 {
        app.update();
    }

    // THE BUG: the placeholder shooter resolves to the "Someone" fallback and the None report
    let texts = log_line_texts(&mut app);
    assert!(
        !texts.iter().any(|t| t == "Someone missed"),
        "a grenade blast must NOT render a phantom \"Someone missed\" combat-log line (a blast \
         has no ganger-shot verdict — its numbers ride the per-ganger wound/injury signals); \
         rendered log: {texts:?}",
    );
}

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::test_support::{test_armor_registry, test_weapon_registry};
use gdtf_battle_sim::{
    cone::{ConeAngle, PriorShots},
    cover::HeightBand,
    ganger::Facing,
    prelude::{CellLevel, Direction, Position, Stance, StanceKind},
    resolve_coarse::ShotInputs,
    sample_cone::ConcentrationP,
    situation::{BattleRegistries, BattleSetup, Situation, setup_battle},
    stability::RecoilGrowth,
    tuning::{GangerStatTuning, RecoilClimb},
};

pub(crate) fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));

    let gangs = gdtf_battle_sim::test_support::test_gang_registry();
    let registry = test_weapon_registry();
    let melee = gdtf_battle_sim::test_support::test_melee_weapon_registry();
    let armor = test_armor_registry();
    let terrain = gdtf_battle_sim::test_support::test_terrain_registry();
    let stat_tuning = GangerStatTuning::default();
    let fallback_floor_cost = gdtf_battle_sim::tuning::CombatTuning::default()
        .move_costs
        .open;
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &registry,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                ),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(setup.is_some(), "setup_battle must succeed on the fixture");
    let setup = setup?;
    app.update();
    Some((app, setup))
}

pub(crate) const fn zero_cone() -> ConeAngle {
    ConeAngle::new(0.0)
}

pub(crate) const fn some_p() -> ConcentrationP {
    ConcentrationP::new(2.0)
}

pub(crate) const fn no_recoil() -> (PriorShots, RecoilClimb, RecoilGrowth) {
    (
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    )
}

pub(crate) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
}

pub(crate) const fn standing_shot(
    shooter_at: CellLevel,
    target_at: CellLevel,
    cover_band: Option<HeightBand>,
    cone: ConeAngle,
) -> ShotInputs {
    let (prior_shots, recoil_climb, recoil_growth) = no_recoil();
    ShotInputs {
        shooter_position: Position::new(shooter_at),
        shooter_facing: Facing::new(Direction::East),
        shooter_stance: Stance::new(StanceKind::Standing),
        target_position: Position::new(target_at),
        target_stance: Stance::new(StanceKind::Standing),
        cover_band,
        cone,
        p: some_p(),
        prior_shots,
        recoil_climb,
        recoil_growth,
    }
}

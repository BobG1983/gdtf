use bevy::ecs::entity::Entity;
use gdtf_battle_sim::{
    acts::FireRequested,
    battle::BattleInProgress,
    cover::HeightBand,
    ganger::{Faction, Hp, LifeState, Tu, TuMax, Wounds},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    test_support::{key, single_mode},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

use super::harness::*;

fn shooter_weapon_kit(mode: FireModeSpec) -> impl bevy::prelude::Bundle {
    let mag_size = MagazineSize::new(30);
    (
        WeaponBundle::new(
            WeaponName::new(String::from("test-weapon")),
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
                Magazine::new(LoadedRounds::new(10), mag_size, ReloadTu::new(12)),
                FireMode::new(vec![mode]),
                Stable::new(true),
                Shove::new(false),
                Handedness::OneHanded,
            ),
        ),
        TuMax::new(100),
    )
}

fn find_ganger(app: &mut bevy::app::App, faction: u8) -> Option<Entity> {
    let wanted = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find_map(|(entity, &f)| (f == wanted).then_some(entity))
}

#[test]
fn fire_requested_in_battle_running_drives_the_sim() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a battle must be set up in BattleRunning (BattleInProgress witness present)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the OccupancyGrid the band-free march reads must be present in BattleRunning",
    );

    let shooter_found = find_ganger(&mut app, SHOOTER_FACTION);
    assert!(
        shooter_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    let target_found = find_ganger(&mut app, TARGET_FACTION);
    assert!(
        target_found.is_some(),
        "the real setup must have spawned a faction-{TARGET_FACTION} target ganger",
    );
    let (Some(shooter), Some(target)) = (shooter_found, target_found) else {
        return;
    };
    assert_ne!(
        shooter, target,
        "the shooter and target are distinct entities"
    );

    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).map(|w| **w);
    let life_before = app.world().get::<LifeState>(target).copied();
    let tu_before = app.world().get::<Tu>(shooter).copied();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(tx, ty),
        Level::new(tl),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).map(|w| **w);
    let life_after = app.world().get::<LifeState>(target).copied();
    let tu_after = app.world().get::<Tu>(shooter).copied();

    let target_changed =
        hp_after != hp_before || wounds_after != wounds_before || life_after != life_before;
    let tu_dropped = matches!((tu_before, tu_after), (Some(b), Some(a)) if *a < *b);
    assert!(
        target_changed || tu_dropped,
        "a FireRequested in BattleRunning must drive the sim — a target component changed or the \
         shooter's Tu dropped (hp {hp_before:?}->{hp_after:?}, wounds {wounds_before:?}->\
         {wounds_after:?}, life {life_before:?}->{life_after:?}, tu {tu_before:?}->{tu_after:?})",
    );
}

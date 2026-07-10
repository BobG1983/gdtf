//! AC1 drive proof: a `FireRequested` in `BattleRunning` drives the sim end-to-end.

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

/// The DETERMINISTIC weapon-state bundle the drive proof RE-ARMS the shooter with —
/// since GTW-257 `setup_battle` arms every ganger from the registry, this OVERWRITES
/// that registry weapon (a second `insert` on the queried-from-`setup_battle` shooter
/// entity, the `acts.rs` AC7 `entity_mut(..).insert(..)` precedent — augmenting an
/// already-spawned entity, NOT spawning a new one) with a precise kit so the test's
/// shot lands in a known regime. It ALSO supplies `TuMax` + `Magazine`, which
/// `setup_battle` does NOT add. Arbitrary magnitudes (not shipped tuning). Mirrors
/// `acts.rs::spawn_shooter`'s weapon kit.
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
            // GTW-275: the WeaponBundle now carries the Magazine grouping, so the kit's
            // known 10-round load rides in the HandlingProfile (a separate Magazine in
            // the same bundle would be a duplicate-component panic).
            HandlingProfile::new(
                Magazine::new(LoadedRounds::new(10), mag_size, ReloadTu::new(12)),
                FireMode::new(vec![mode]),
                Stable::new(true),
                Shove::new(false),
                Handedness::OneHanded,
            ),
        ),
        // The shooter query also reads TuMax — not authored by `setup_battle` — so the
        // kit supplies it (the Magazine is now part of the WeaponBundle above).
        TuMax::new(100),
    )
}

/// Find the spawned ganger entity of `faction` in the world (the real-path query the
/// drive proof uses instead of hand-spawning). Returns the FIRST match — the fixture
/// authors exactly one ganger per faction. Runs entirely off `app.world_mut()` (the
/// accepted test-body idiom): it spawns nothing and takes no `&mut World` helper param.
fn find_ganger(app: &mut bevy::app::App, faction: u8) -> Option<Entity> {
    let wanted = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find_map(|(entity, &f)| (f == wanted).then_some(entity))
}

/// AC1 — DRIVE PROOF: a `FireRequested` emitted while in `BattleRunning` drives the sim
/// (the E10.5-bundled, witness-gated dispatch band is live in `BattleRunning`).
///
/// Sets the battle up via the REAL message-driven path: inserts a two-ganger
/// `LoadedSituation`, drives to `BattleRunning` (the app's `OnEnter(Generation)` sends
/// `SetupBattleRequested`, the sim's `setup_battle` spawns both gangers via `Commands`),
/// then QUERIES the spawned shooter + target entities by faction. It ARMS the queried
/// shooter by `insert`-ing the weapon kit a `GangerSpawn` does not author (the
/// `entity_mut(..).insert(..)` augment-an-existing-entity precedent), PUBLISHES the
/// target's occupant band in the live `OccupancyGrid` (the silhouette the band-free march
/// reads to resolve a `Ganger` hit), captures the target's baseline, emits a
/// `FireRequested` inline via the world message buffer, `update()`s once, and asserts ≥1
/// queried target component (`Hp`/`Wounds`/`LifeState`) changed OR the shooter's `Tu`
/// dropped (a seed-dependent miss still charges TU) — a relation, never a pinned
/// magnitude. No `&mut World` helper; no hand-spawn that bypasses `setup_battle`.
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

    // The battle was poured into the world by the real Generation setup — the witness the
    // bundled dispatch band gates on (GTW-212's BattleInProgress, no longer OccupancyGrid)
    // is present, and the OccupancyGrid the march reads is too.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a battle must be set up in BattleRunning (BattleInProgress witness present)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the OccupancyGrid the band-free march reads must be present in BattleRunning",
    );

    // Query the two SETUP-SPAWNED gangers (never hand-spawned): the shooter (faction 0)
    // and the target (faction 1). The real setup must have spawned exactly one of each.
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

    // ARM the queried shooter — a `GangerSpawn` authors no weapon (E10.3 added only armor),
    // so `insert` the weapon kit onto the EXISTING setup-spawned entity (augment, never
    // re-spawn). This is the only way the bundled `dispatch_fire` (which needs
    // `With<Weapon>` + every weapon stat) resolves a real shot from a setup-spawned shooter.
    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    // PUBLISH the target's occupant band in the live grid — the band-free march reads it
    // to band the round vs the occupant (a `Ganger` hit needs both occupant + band, and
    // neither `setup_battle` nor the maintenance layer publishes the band). HIGH so a
    // standing target is squarely in the round's path.
    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).map(|w| **w);
    let life_before = app.world().get::<LifeState>(target).copied();
    let tu_before = app.world().get::<Tu>(shooter).copied();

    // Emit the fire request IN BattleRunning, then advance one update so the bundled
    // dispatch band (live battle-wide) consumes it.
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

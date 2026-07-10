//! The bootstrap situation, the seeded driven app, and shared lookups.

use bevy::ecs::entity::Entity;
use gdtf_battle_sim::{
    ganger::{Facing, Faction, Stance, StanceKind, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    metric::CellLevel,
    rng::BattleSeed,
    situation::Situation,
    test_support::{GangerSpawnBuilder, SituationBuilder, key},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::BattleAppBuilder;

/// The shooter's faction in the fixture (the ganger the drive proof arms + fires).
pub(crate) const SHOOTER_FACTION: u8 = 0;
/// The target's faction in the fixture (the ganger fired at).
pub(crate) const TARGET_FACTION: u8 = 1;

/// The `(cell, level)` the shooter is authored at — west of the target, same storey,
/// so a due-East shot reaches it.
pub(crate) const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
/// The `(cell, level)` the target is authored at — due East of the shooter, close
/// range so the shot lands deterministically under the fixed seed.
pub(crate) const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

/// The stance every fixture ganger is authored holding — the AC3 "authored start" value
/// the requested stance must differ from. Matches the
/// [`GangerSpawnBuilder`](gdtf_battle_sim::test_support::GangerSpawnBuilder) default.
pub(crate) const AUTHORED_STANCE: StanceKind = StanceKind::Standing;

/// Build the bootstrap fixture's authored ganger at `at` / `faction` via the central
/// [`GangerSpawnBuilder`](gdtf_battle_sim::test_support::GangerSpawnBuilder): facing
/// East (so a due-East shot reaches the target), holding the [`AUTHORED_STANCE`],
/// referencing the central test weapon + armor keys (the default), so a setup arms +
/// armors it from the registries the [`BattleAppBuilder`] seeds.
pub(crate) fn bootstrap_ganger(
    at: CellLevel,
    faction: u8,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(gdtf_battle_sim::ganger::Direction::East))
        .stance(Stance::new(AUTHORED_STANCE))
        .build()
}

/// The bootstrap fixture: a shooter (faction [`SHOOTER_FACTION`]) facing East and a
/// target (faction [`TARGET_FACTION`]) directly East at close range, built over the
/// central [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder).
/// Link-free, so the setup validates trivially. The `SetupBattleRequested` the app
/// sends on `OnEnter(Generation)` pours this real battle into the world before
/// `BattleRunning`.
pub(crate) fn bootstrap_situation() -> Situation {
    let (sx, sy, sl) = SHOOTER_AT;
    let (tx, ty, tl) = TARGET_AT;
    SituationBuilder::new()
        .with_gangers([
            bootstrap_ganger(key(sx, sy, sl), SHOOTER_FACTION),
            bootstrap_ganger(key(tx, ty, tl), TARGET_FACTION),
        ])
        .build()
}

/// The fixed [`BattleSeed`] every bootstrap test run uses — the same as the
/// pre-GTW-14 `DEFAULT_BATTLE_SEED = BattleSeed::new(0)` constant, kept as `0`
/// so the seeded drive is guaranteed to land the shot at close range.
pub(crate) const BOOTSTRAP_SEED: BattleSeed = BattleSeed::new(0);

/// Build the bootstrap app already driven to a live battle via the shared
/// [`BattleAppBuilder`], seeded with the bootstrap [`bootstrap_situation`] and the
/// fixed [`BOOTSTRAP_SEED`] (guarantees the shot lands at close range and keeps
/// every test deterministic). Returns `None` if the shared drive does not reach
/// `BattleRunning` (the caller asserts).
pub(crate) fn bootstrap_app() -> Option<bevy::app::App> {
    BattleAppBuilder::new()
        .with_situation(bootstrap_situation())
        .with_seed(BOOTSTRAP_SEED)
        .build()
}

/// The DETERMINISTIC weapon-state bundle the drive proof RE-ARMS the shooter with —
/// since GTW-257 `setup_battle` arms every ganger from the registry, this OVERWRITES
/// that registry weapon (via a second `insert` on the queried-from-`setup_battle`
/// shooter entity) with a precise, high-damage kit so the test's single shot lands in a
/// known regime. It ALSO supplies `TuMax`, which `setup_battle` does NOT add (the
/// `WeaponBundle` carries the `Magazine` grouping itself since GTW-275). Arbitrary
/// magnitudes (not shipped tuning). Mirrors the `acts.rs` weapon kit.
pub(crate) fn shooter_weapon_kit(mode: FireModeSpec) -> impl bevy::prelude::Bundle {
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
            // known 10-round load rides in the HandlingProfile (a separate Magazine in the
            // same bundle would be a duplicate-component panic).
            HandlingProfile::new(
                Magazine::new(LoadedRounds::new(10), mag_size, ReloadTu::new(12)),
                FireMode::new(vec![mode]),
                Stable::new(true),
                Shove::new(false),
                Handedness::OneHanded,
            ),
        ),
        // The shooter query also reads TuMax — not authored by `setup_battle` — so the kit
        // supplies it (the Magazine is now part of the WeaponBundle above).
        TuMax::new(100),
    )
}

/// Find the spawned ganger entity of `faction` in the world (the real-path query the
/// drive proof uses instead of hand-spawning). Returns the FIRST match — the fixture
/// authors exactly one ganger per faction. Runs entirely off `app.world_mut()` (the
/// accepted test-body idiom): it spawns nothing and takes no `&mut World` helper param.
pub(crate) fn find_ganger(app: &mut bevy::app::App, faction: u8) -> Option<Entity> {
    let wanted = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find_map(|(entity, &f)| (f == wanted).then_some(entity))
}

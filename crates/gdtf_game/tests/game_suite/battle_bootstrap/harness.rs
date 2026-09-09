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
use gdtf_game::test_support::BattleAppBuilder;

pub(crate) const SHOOTER_FACTION: u8 = 0;
pub(crate) const TARGET_FACTION: u8 = 1;

pub(crate) const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
pub(crate) const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

pub(crate) const AUTHORED_STANCE: StanceKind = StanceKind::Standing;

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

pub(crate) const BOOTSTRAP_SEED: BattleSeed = BattleSeed::new(0);

pub(crate) fn bootstrap_app() -> bevy::app::App {
    BattleAppBuilder::new()
        .with_situation(bootstrap_situation())
        .with_seed(BOOTSTRAP_SEED)
        .build()
}

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

pub(crate) fn find_ganger(app: &mut bevy::app::App, faction: u8) -> Option<Entity> {
    let wanted = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find_map(|(entity, &f)| (f == wanted).then_some(entity))
}

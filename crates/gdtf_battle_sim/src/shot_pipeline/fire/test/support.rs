pub(super) use bevy::{
    ecs::system::SystemState,
    prelude::{Entity, World},
};

pub(super) use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    fire::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery,
        Volley, WeaponQuery, WearsQuery, WieldsQuery, fire,
    },
    ganger::{
        Aiming, Direction, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, StanceKind,
        Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::{InflictedWound, InflictedWounds},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{LoadedRounds, Magazine, ReloadTu, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{AppliedDamage, GangerVerdict, HitReport, HitVerdict},
    resolve_coarse::ShotKind,
    rng::{InjuryRng, SeverityRng, ShotRng},
    severity::Severity,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    test_support::{single_mode, target_bundle},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FightMode, FightModeKind,
        FightModeSpec, FireMode, FireModeSpec, Handedness, HandlingProfile, Kickback, MagazineSize,
        MeleeDamageProfile, MeleeWeaponBundle, Reach, Shove, Stable, Strikes, TuCost, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

pub(super) type FireQueries = (
    ShooterQuery<'static, 'static>,
    TargetQuery<'static, 'static>,
    WearsQuery<'static, 'static>,
    PieceQuery<'static, 'static>,
    WieldsQuery<'static, 'static>,
    WeaponQuery<'static, 'static>,
    MeleeQuery<'static, 'static>,
    MountedQuery<'static, 'static>,
);

pub(super) const SEED: u64 = 0xF12E_5EED;

pub(super) fn ganger_verdict(report: &HitReport) -> Option<&GangerVerdict> {
    match &report.verdict {
        HitVerdict::Ganger(verdict) => Some(verdict),
        HitVerdict::Cover(_)
        | HitVerdict::Slab(_)
        | HitVerdict::Ground(_)
        | HitVerdict::NoEffect => None,
    }
}

pub(super) fn applied_of(report: &HitReport) -> Option<AppliedDamage> {
    ganger_verdict(report).map(|verdict| verdict.applied)
}

pub(super) fn rng() -> ShotRng {
    crate::test_support::shot_rng(SEED)
}

pub(super) fn severity_rng() -> SeverityRng {
    crate::test_support::severity_rng(SEED)
}

pub(super) fn injury_rng() -> InjuryRng {
    crate::test_support::injury_rng(SEED)
}

pub(super) fn injury_tables() -> InjuryTables {
    InjuryTables::default()
}

pub(super) fn injury_registry() -> InjuryRegistry {
    InjuryRegistry::default()
}

pub(super) fn equip_uniform_armor(
    world: &mut World,
    ganger: Entity,
    floor: i32,
    protection: i32,
    integrity: i32,
    hardness: i32,
) {
    for part in BodyPart::ALL {
        world.spawn((
            WornBy::new(ganger),
            part,
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(hardness),
            ArmorType::DEFAULT,
        ));
    }
}

pub(super) fn weapon_rounds(world: &World, ganger: Entity) -> Option<u16> {
    let wields = world.get::<crate::weapon::Wields>(ganger)?;
    let weapon = wields.weapon()?;
    world.get::<Magazine>(weapon).map(|m| *m.rounds())
}

pub(super) fn equip_weapon(world: &mut World, ganger: Entity, weapon: WeaponBundle) {
    world.spawn((WieldedBy::new(ganger), weapon));
}

pub(super) fn equip_melee_weapon(world: &mut World, ganger: Entity) {
    let melee = MeleeWeaponBundle::new(
        WeaponName::new("test-melee".to_owned()),
        MeleeDamageProfile::new(
            WeaponDamage::new(9),
            WeaponPunch::new(3),
            WeaponShred::new(8),
            DamageType::Rend,
        ),
        FatalBias::new(4.0),
        Handedness::OneHanded,
        Reach::new(1),
        FightMode::new(vec![FightModeSpec::new(
            FightModeKind::Swing,
            TuCost::new(20),
            Strikes::new(1),
        )]),
        Shove::new(false),
    );
    world.spawn((WieldedBy::new(ganger), melee));
}

fn test_weapon(
    ammo: u16,
    base_spread: f32,
    kickback: f32,
    stable: bool,
    mode: FireModeSpec,
) -> WeaponBundle {
    let mag_size = MagazineSize::new(30);
    let reload_tu = ReloadTu::new(12);
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(base_spread),
        Accuracy::new(2.0),
        Kickback::new(kickback),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(LoadedRounds::new(ammo), mag_size, reload_tu),
            FireMode::new(vec![mode]),
            Stable::new(stable),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

pub(super) fn equip_handed_weapon(world: &mut World, ganger: Entity, handedness: Handedness) {
    let mode = single_mode(0.2, 1);
    let weapon = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
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
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            handedness,
        ),
    );
    world.spawn((WieldedBy::new(ganger), weapon));
}

pub(super) fn give_disabled_hand(world: &mut World, ganger: Entity, part: BodyPart) {
    use crate::injuries::{GainedInjury, InflictedInjuries, InjuryEffect, InjuryName, InspectText};
    let mut ledger = InflictedInjuries::default();
    ledger.gain(GainedInjury::new(
        InjuryName::new("disabled-hand".to_owned()),
        part,
        Severity::Major,
        vec![InjuryEffect::DisableHand],
        InspectText::new("a disabled hand".to_owned()),
    ));
    world.entity_mut(ganger).insert(ledger);
}

#[derive(Clone)]
pub(super) struct ShooterSpec {
    pub(super) x:      i32,
    pub(super) y:      i32,
    pub(super) tu:     u8,
    pub(super) tu_max: u8,
    pub(super) ammo:   u16,
    pub(super) mode:   FireModeSpec,
    pub(super) aiming: bool,
}

pub(super) fn spawn_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
    let ammo = spec.ammo;
    let mode = spec.mode;
    let shooter = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(spec.x, spec.y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(spec.aiming),
            Shooting::new(1.0),
            Tu::new(spec.tu),
            TuMax::new(spec.tu_max),
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    equip_weapon(world, shooter, test_weapon(ammo, 0.05, 0.2, true, mode));
    equip_uniform_armor(world, shooter, 0, 0, 1, 0);
    shooter
}

pub(super) fn spawn_zero_spread_shooter(world: &mut World, spec: ShooterSpec) -> Entity {
    let ammo = spec.ammo;
    let mode = spec.mode;
    let shooter = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(spec.x, spec.y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(spec.aiming),
            Shooting::new(1.0),
            Tu::new(spec.tu),
            TuMax::new(spec.tu_max),
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    equip_weapon(world, shooter, test_weapon(ammo, 0.0, 0.4, false, mode));
    equip_uniform_armor(world, shooter, 0, 0, 1, 0);
    shooter
}

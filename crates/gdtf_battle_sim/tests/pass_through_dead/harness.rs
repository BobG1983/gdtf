use bevy::{
    ecs::system::SystemState,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::{CoverLedger, HeightBand},
    fire::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, Volley,
        WeaponQuery, WearsQuery, WieldsQuery,
    },
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{
        Cell, CellLevel, Direction, Level, LifeState, OccupancyGrid, Position, Stance, StanceKind,
        Tu,
    },
    resolve_coarse::ShotKind,
    slab::BraceStairCells,
    surface::SurfaceGrid,
    test_support::{GangerEntityBuilder, empty_slab_ledger, injury_rng, severity_rng, shot_rng},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy,
    },
};

pub(crate) fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), Level::new(0))
}

pub(crate) fn front_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), Level::new(0))
}

pub(crate) fn behind_cell() -> CellLevel {
    CellLevel::new(Cell::new(9, 5), Level::new(0))
}

pub(crate) const fn burst_mode(shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(shots),
    )
}

pub(crate) fn equip_thin_armor(world: &mut World, ganger: Entity) {
    for part in BodyPart::ALL {
        world.spawn((
            WornBy::new(ganger),
            part,
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(1),
            ArmorHardness::new(0),
            ArmorType::DEFAULT,
        ));
    }
}

pub(crate) fn spawn_shooter(world: &mut World, mode: FireModeSpec) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("probe-weapon".to_owned()),
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(60),
            WeaponPunch::new(40),
            WeaponShred::new(20),
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
            Handedness::OneHanded,
        ),
    );
    let shooter = world
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(250),
            TuMax::new(100),
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
    world.spawn((WieldedBy::new(shooter), bundle));
    equip_thin_armor(world, shooter);
    shooter
}

pub(crate) fn line_ganger(
    world: &mut World,
    cell: CellLevel,
    wounds: u8,
    state: LifeState,
) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .at(cell)
        .stance(StanceKind::Standing)
        .combat_vitals(40, wounds)
        .life_state(state)
        .spawn(world);
    equip_thin_armor(world, ganger);
    ganger
}

pub(crate) fn place_occupant(occupancy: &mut OccupancyGrid, cell: CellLevel, entity: Entity) {
    occupancy.set_occupant(cell, Some(entity));
    occupancy.set_occupant_band(cell, Some(HeightBand::High));
}

pub(crate) fn fire_volley(
    world: &mut World,
    shooter: Entity,
    mode: FireModeSpec,
    occupancy: &OccupancyGrid,
    seed: u64,
) -> Volley {
    type FireQueries<'w, 's> = (
        ShooterQuery<'w, 's>,
        gdtf_battle_sim::fire::TargetQuery<'w, 's>,
        WearsQuery<'w, 's>,
        PieceQuery<'w, 's>,
        WieldsQuery<'w, 's>,
        WeaponQuery<'w, 's>,
        MeleeQuery<'w, 's>,
        MountedQuery<'w, 's>,
    );
    let tuning = CombatTuning::default();
    let mut rng = shot_rng(seed);
    let mut sev_rng = severity_rng(seed);
    let mut injury_rng = injury_rng(seed);
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = empty_slab_ledger();

    let mut state: SystemState<FireQueries> = SystemState::new(world);
    let access = state.get_mut(world);
    assert!(access.is_ok(), "shooter/target queries must validate");
    let volley = match access {
        Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) => {
            gdtf_battle_sim::fire::fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(front_cell().x, front_cell().y),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                &wears,
                &mut pieces,
                &wields,
                &mut weapons,
                &melee,
                &mounted,
                BattleGrids {
                    occupancy,
                    surface: &surface,
                    cover: &mut cover,
                    slab: &mut slab,
                    brace_cells: &BraceStairCells::empty(),
                },
                &tuning,
                &mut rng,
                &mut sev_rng,
                &InjuryTables::default(),
                &InjuryRegistry::default(),
                &mut injury_rng,
            )
        }
        Err(_) => Volley {
            reports: Vec::new(),
            shots:   Vec::new(),
            splash:  Vec::new(),
        },
    };
    state.apply(world);
    volley
}

pub(crate) fn report_struck(volley: &Volley, entity: Entity) -> bool {
    volley
        .reports
        .iter()
        .any(|r| r.kind == ShotKind::Ganger(entity))
}

pub(crate) fn applied_on(
    volley: &Volley,
    entity: Entity,
) -> Option<gdtf_battle_sim::resolve_and_apply::AppliedDamage> {
    volley
        .reports
        .iter()
        .find(|r| r.kind == ShotKind::Ganger(entity))
        .and_then(|r| match &r.verdict {
            gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger(verdict) => {
                Some(verdict.applied)
            }
            _ => None,
        })
}

pub(crate) fn struck_count(volley: &Volley, entity: Entity) -> usize {
    volley
        .reports
        .iter()
        .filter(|r| r.kind == ShotKind::Ganger(entity))
        .count()
}

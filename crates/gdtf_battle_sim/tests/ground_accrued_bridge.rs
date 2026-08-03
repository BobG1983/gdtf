use bevy::{
    app::App,
    math::Vec3,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    cover::CoverLedger,
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    march::{MarchDir, MarchKind, march_vector},
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, SimPos,
        Stance, StanceKind, Tu,
    },
    slab::SlabLedger,
    surface::SurfaceGrid,
    test_support::{SimAppBuilder, single_mode},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

const PER_ROUND_DAMAGE: i32 = 25;

const ROUNDS: u32 = 5;

fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(4, 4), Level::new(1))
}

const fn ground_cell() -> Cell {
    Cell::new(4, 4)
}

const fn aim_cell() -> Cell {
    Cell::new(4, 4)
}

fn bridge_app() -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(0x0660_0366)
        .with_acts()
        .with_player_faction(1)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

fn spawn_shooter(world: &mut World) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("dirt-kicker".to_owned()),
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(PER_ROUND_DAMAGE),
            WeaponPunch::new(10),
            WeaponShred::new(4),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(
                LoadedRounds::new(30),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![single_mode(0.2, 1)]),
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
            TuMax::new(10),
            Faction::new(1),
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
    shooter
}

fn probe_strikes_ground(
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> bool {
    let muzzle = SimPos::new(4.5, 4.5, 1.5);
    let dir = Vec3::new(0.0, 0.0, -1.0);
    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        occupancy,
        surface,
        cover,
        tuning,
        shooter_cell(),
        |_| false,
    );
    matches!(result.kind, MarchKind::Ground)
}

fn fire_one_round(app: &mut App, shooter: Entity) {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter) {
        *tu = Tu::new(250);
    }
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        aim_cell(),
        Level::new(0),
    ));
    app.update();
    app.update();
}

#[test]
fn fired_rounds_accrue_ground_damage_monotonically_without_touching_other_state() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());

    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    let tuning = CombatTuning::default();
    let cover_probe = CoverLedger::new();
    let cell = ground_cell();

    {
        let surface_res = app.world().resource::<SurfaceGrid>();
        let occ = app.world().resource::<OccupancyGrid>();
        assert!(
            probe_strikes_ground(occ, surface_res, &cover_probe, &tuning),
            "BEFORE: a probe ray fired DOWN must strike the GROUND (the open column the \
             fired round descends through)",
        );
        assert_eq!(
            *surface_res.ground_damage(&cell),
            0,
            "BEFORE: the target cell's ground accumulator must read zero (no strike yet)",
        );
    }

    let hp_before = *app
        .world()
        .get::<Hp>(shooter)
        .copied()
        .unwrap_or(Hp::new(0));
    let wounds_before = *app
        .world()
        .get::<Wounds>(shooter)
        .copied()
        .unwrap_or(Wounds::new(0));

    let mut last_total = 0_u32;
    for n in 1..=ROUNDS {
        fire_one_round(&mut app, shooter);
        let total = *app.world().resource::<SurfaceGrid>().ground_damage(&cell);
        assert!(
            total > last_total,
            "C7(b): the ground accumulator must STRICTLY GROW each strike (monotonic) — \
             round {n}: {total} vs previous {last_total}",
        );
        let expected_sum = u32::try_from(PER_ROUND_DAMAGE).unwrap_or(0) * n;
        assert_eq!(
            total, expected_sum,
            "C7(a/b): after {n} ground strikes the accumulator must equal the SUM of the \
             rounds' weapon_damage ({expected_sum})",
        );
        last_total = total;
    }

    let final_total = *app.world().resource::<SurfaceGrid>().ground_damage(&cell);
    assert_eq!(
        final_total,
        u32::try_from(PER_ROUND_DAMAGE).unwrap_or(0) * ROUNDS,
        "C7(a): the final ground total must be the summed weapon_damage of the whole barrage",
    );

    assert_eq!(
        *app.world()
            .get::<Hp>(shooter)
            .copied()
            .unwrap_or(Hp::new(0)),
        hp_before,
        "C7(c): a ground barrage must not change the shooter's HP (it wounds no ganger)",
    );
    assert_eq!(
        *app.world()
            .get::<Wounds>(shooter)
            .copied()
            .unwrap_or(Wounds::new(0)),
        wounds_before,
        "C7(c): a ground barrage must not change any ganger's Wounds",
    );
    assert!(
        app.world()
            .resource::<CoverLedger>()
            .peek(&CellLevel::new(cell, Level::new(0)))
            .is_none(),
        "C7(c): a ground barrage must not insert / deplete the cover ledger",
    );
    assert!(
        app.world()
            .resource::<SlabLedger>()
            .peek(&CellLevel::new(cell, Level::new(0)))
            .is_none(),
        "C7(c): a ground barrage must not insert / deplete the slab ledger",
    );
}

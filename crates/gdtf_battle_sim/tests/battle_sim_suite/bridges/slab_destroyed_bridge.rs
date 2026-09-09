//! Destroyed-slab bridge: fire depletes slab HP, opens LOS, keeps walkability closed.

use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::CoverLedger,
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    march::{MarchDir, MarchGrids, MarchKind, march_vector},
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, SimPos,
        Stance, StanceKind, Tu,
    },
    slab::{BraceStairCells, SlabEntry, SlabHp},
    surface::{SlabState, SurfaceGrid},
    test_support::{SimAppBuilder, empty_slab_ledger, single_mode},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(0))
}

fn slab_key() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(1))
}

const fn aim_cell() -> Cell {
    Cell::new(5, 5)
}

fn bridge_app() -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(0x51AB_C0DE)
        .with_acts()
        .with_player_faction(1)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

fn spawn_shooter(world: &mut World) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("chipper".to_owned()),
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(30),
            WeaponShred::new(10),
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

const fn low_hp_slab() -> SlabEntry {
    SlabEntry::seeded(
        SlabHp::new(120),
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

fn probe_stops_on_slab(
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> bool {
    let muzzle = SimPos::new(5.5, 5.5, 0.5);
    let dir = bevy::math::Vec3::new(0.0, 0.0, 1.0);
    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy,
            surface,
            cover,
        },
        tuning,
        shooter_cell(),
        |_| false,
    );
    matches!(result.kind, MarchKind::Slab)
}

fn fire_one_round(app: &mut App, shooter: Entity) {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter) {
        *tu = Tu::new(250);
    }
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        aim_cell(),
        Level::new(1),
    ));
    app.update();
    app.update();
}

#[test]
fn fired_rounds_deplete_then_destroy_slab_and_open_los_without_walkability() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());

    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key(), SlabState::Present);
    app.insert_resource(surface);
    let mut slab = empty_slab_ledger();
    slab.insert(slab_key(), low_hp_slab());
    app.insert_resource(slab);
    app.insert_resource(BraceStairCells::empty());
    app.insert_resource(OccupancyGrid::new());
    let links_before = app.world().resource::<VerticalLinkGraph>().links().count();
    assert!(
        app.world()
            .resource::<VerticalLinkGraph>()
            .links_from(&slab_key())
            .next()
            .is_none(),
        "BEFORE: the intact slab key has no departing vertical link (the C9(c) baseline)",
    );

    let tuning = CombatTuning::default();
    let cover_probe = CoverLedger::new();

    {
        let surface_res = app.world().resource::<SurfaceGrid>();
        assert_eq!(
            surface_res.slab_state(&slab_key()),
            SlabState::Present,
            "BEFORE: the slab must be intact (Present)",
        );
        let occ = app.world().resource::<OccupancyGrid>();
        assert!(
            probe_stops_on_slab(occ, surface_res, &cover_probe, &tuning),
            "BEFORE: a probe ray fired UP must STOP on the intact slab (it blocks rounds + LOS)",
        );
    }

    fire_one_round(&mut app, shooter);
    assert_eq!(
        app.world()
            .resource::<SurfaceGrid>()
            .slab_state(&slab_key()),
        SlabState::Present,
        "C9(a): one low-damage round must only DAMAGE the slab — it stays Present (the pool \
         persists above zero)",
    );

    let mut strikes_to_destroy = 1_u32;
    loop {
        if app
            .world()
            .resource::<SurfaceGrid>()
            .slab_state(&slab_key())
            == SlabState::Absent
        {
            break;
        }
        fire_one_round(&mut app, shooter);
        strikes_to_destroy += 1;
    }

    assert!(
        strikes_to_destroy > 1,
        "C9(a): the slab must deplete over MULTIPLE persistent strikes, not one — took \
         {strikes_to_destroy}",
    );
    assert_eq!(
        app.world()
            .resource::<SurfaceGrid>()
            .slab_state(&slab_key()),
        SlabState::Absent,
        "C9(a): the bridge (dispatch_fire → TerrainPieceDestroyed → replace_destroyed_piece) \
         must leave the cell with no slab standing once its persistent pool hits zero, because \
         this app holds no def registry and so nothing is left behind",
    );

    {
        let surface_res = app.world().resource::<SurfaceGrid>();
        let occ = app.world().resource::<OccupancyGrid>();
        assert!(
            !probe_stops_on_slab(occ, surface_res, &cover_probe, &tuning),
            "C9(b): a probe ray fired UP must now PASS THROUGH the smashed slab (rounds + LOS \
             pass — the shared march stops only on SlabState::Present)",
        );
    }

    let links_after = app.world().resource::<VerticalLinkGraph>().links().count();
    assert_eq!(
        links_before, links_after,
        "C9(c): destroying a slab must NOT change the vertical-link graph's link count (a \
         destroyed slab is NOT walkable — no pathfinding / vertical-link change)",
    );
    assert!(
        app.world()
            .resource::<VerticalLinkGraph>()
            .links_from(&slab_key())
            .next()
            .is_none(),
        "C9(c): a destroyed slab must add NO vertical link from its key (not walkable)",
    );
}

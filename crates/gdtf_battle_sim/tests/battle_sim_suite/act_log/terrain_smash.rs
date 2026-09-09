//! A destroyed terrain piece reaches the act log carrying which kind it was.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    act_log::ActDeed,
    acts::{FireRequested, MeleeRequested},
    armor::{ArmorHardness, ArmorProtection},
    cover::CoverLedger,
    entity::TerrainPieceKind,
    ganger::{Aiming, Direction, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position, Stance, StanceKind, Tu},
    slab::{SlabEntry, SlabHp, SlabLedger},
    surface::{SlabState, SurfaceGrid},
    test_support::{SituationBuilder, single_mode},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

use super::harness::*;

fn shooter_cell() -> CellLevel {
    ground(5, 5)
}

fn slab_key() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(1))
}

fn spawn_shooter(app: &mut App) -> Entity {
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
    let world = app.world_mut();
    let shooter = world
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(250),
            TuMax::new(10),
            Faction::new(ENEMY),
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

fn seed_slab(app: &mut App) {
    if let Some(mut surface) = app.world_mut().get_resource_mut::<SurfaceGrid>() {
        surface.set_slab(slab_key(), SlabState::Present);
    }
    if let Some(mut slab) = app.world_mut().get_resource_mut::<SlabLedger>() {
        slab.insert(slab_key(), low_hp_slab());
    }
}

fn slab_state(app: &App) -> Option<SlabState> {
    app.world()
        .get_resource::<SurfaceGrid>()
        .map(|surface| surface.slab_state(&slab_key()))
}

fn fire_one_round(app: &mut App, shooter: Entity) {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter) {
        *tu = Tu::new(250);
    }
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(5, 5),
        Level::new(1),
    ));
    app.update();
    app.update();
}

#[test]
fn a_slab_shot_to_pieces_records_one_smash_deed_naming_the_slab() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let situation = SituationBuilder::new()
        .with_gangers([watcher(ground(0, 0), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let shooter = spawn_shooter(&mut app);
    seed_slab(&mut app);

    loop {
        if slab_state(&app) == Some(SlabState::Absent) {
            break;
        }
        fire_one_round(&mut app, shooter);
    }
    assert_eq!(
        slab_state(&app),
        Some(SlabState::Absent),
        "precondition: the fire path must destroy the slab, or the deed assertion is vacuous",
    );

    let deeds = deeds_of(&app, "TerrainPieceSmashed");
    assert_eq!(
        deeds.len(),
        1,
        "destroying the slab records exactly one smash deed — found {} deed(s): {deeds:?}",
        deeds.len(),
    );
    assert_eq!(
        deeds.first(),
        Some(&ActDeed::TerrainPieceSmashed {
            at:   slab_key(),
            kind: TerrainPieceKind::Slab,
        }),
        "the recorded smash names the slab cell and the Slab kind — found {deeds:?}",
    );
}

fn attacker_cell() -> CellLevel {
    ground(5, 5)
}

fn cover_cell() -> CellLevel {
    ground(6, 5)
}

fn cover_destroyed(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .is_some_and(|entry| *entry.destroyed)
}

#[test]
fn a_cover_smashed_in_melee_records_one_smash_deed_naming_the_cover() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let situation = SituationBuilder::new()
        .with_gangers([watcher(attacker_cell(), PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    seed_cover(&mut app, cover_cell(), 60, TerrainPieceKind::Cover);
    let Some(attacker) = ganger_at(&mut app, attacker_cell()) else {
        unreachable!("setup spawns the attacker at its authored cell");
    };

    loop {
        if cover_destroyed(&app, cover_cell()) {
            break;
        }
        set_tu(&mut app, attacker, 250);
        app.world_mut()
            .write_message(MeleeRequested::new_structural(attacker, cover_cell()));
        settle(&mut app);
    }
    assert!(
        cover_destroyed(&app, cover_cell()),
        "precondition: the melee path must destroy the cover, or the deed assertion is vacuous",
    );

    let deeds = deeds_of(&app, "TerrainPieceSmashed");
    assert_eq!(
        deeds.len(),
        1,
        "smashing the cover to pieces records exactly one smash deed — found {} deed(s): \
         {deeds:?}",
        deeds.len(),
    );
    assert_eq!(
        deeds.first(),
        Some(&ActDeed::TerrainPieceSmashed {
            at:   cover_cell(),
            kind: TerrainPieceKind::Cover,
        }),
        "the recorded smash names the cover cell and the Cover kind — found {deeds:?}",
    );
}

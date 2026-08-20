use bevy::{app::App, ecs::relationship::RelationshipTarget as _, prelude::Entity};
use gdtf_battle_sim::{
    acts::EnterEmplacementRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Aiming, LifeState, Suppressed, SuppressorCell, TuMax},
    magazine::{LoadedRounds, Magazine, mode_tu_cost},
    metric::CellLevel,
    prelude::{Position, Tu},
    terrain::{
        emplacement::{
            EmplacementEntrySides, EmplacementFacing, EmplacementState, MountedWeaponEntity,
            MountedWeaponKey,
        },
        entity::TerrainCell,
        facing::TerrainFacing,
    },
    test_support::{TEST_WEAPON_KEY, test_weapon_spec},
    tuning::CombatTuning,
    weapon::{FireMode, WeaponName, WeaponRegistry, Wields},
};

use super::harness::{cycle_back_to_player_turn, pos_of, step, tu_of, used_of};

pub(crate) const MOUNTED_KEY: &str = "test-mounted-gun";

pub(crate) fn two_gun_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(TEST_WEAPON_KEY.to_owned()),
            test_weapon_spec(),
        ),
        (WeaponName::new(MOUNTED_KEY.to_owned()), test_weapon_spec()),
    ])
}

/// An emplacement enterable from every side, with the cover block a seeded one would carry.
pub(crate) fn spawn_emplacement(app: &mut App, at: CellLevel) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            TerrainCell::new(at),
            EmplacementState::Vacant,
            MountedWeaponKey::new(WeaponName::new(MOUNTED_KEY.to_owned())),
            EmplacementEntrySides::new(TerrainFacing::ALL.to_vec()),
            EmplacementFacing::new(TerrainFacing::default()),
        ))
        .id();
    app.world_mut().resource_mut::<CoverLedger>().insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(45),
            HeightBand::High,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
            TerrainPieceKind::Emplacement,
        ),
    );
    entity
}

/// Seat `ganger` through the real enter act and report the mount it spawned.
pub(crate) fn man_emplacement(app: &mut App, ganger: Entity, emplacement: Entity) -> Entity {
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(ganger, emplacement));
    step(app, 3);
    let Some(mount) = app
        .world()
        .get::<MountedWeaponEntity>(emplacement)
        .map(|m| **m)
    else {
        unreachable!("the enter act spawns + records the mounted weapon");
    };
    mount
}

pub(crate) fn empty_magazine(app: &mut App, weapon: Entity) {
    let Some(magazine) = app.world().get::<Magazine>(weapon).copied() else {
        unreachable!("the weapon entity carries a Magazine");
    };
    let Some(mut live) = app.world_mut().get_mut::<Magazine>(weapon) else {
        unreachable!("the weapon entity carries a Magazine");
    };
    *live = Magazine::new(LoadedRounds::new(0), magazine.size(), magazine.reload_tu());
}

pub(crate) fn carried_gun_loaded(app: &mut App, ganger: Entity, mount: Entity) -> bool {
    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(ganger) else {
            unreachable!("the fixture ganger wields weapons at setup");
        };
        wields.iter().collect()
    };
    wielded.into_iter().any(|entity| {
        entity != mount
            && world
                .get::<Magazine>(entity)
                .is_some_and(|magazine| !*magazine.is_empty())
    })
}

pub(crate) fn is_alive(app: &App, entity: Entity) -> bool {
    matches!(app.world().get::<LifeState>(entity), Some(LifeState::Alive))
}

/// Give the mover a full pool, so the measured move is not gated by what its own turn spent.
fn ready_the_mover(app: &mut App, mover: Entity) {
    let Some(full) = app.world().get::<TuMax>(mover).map(|max| **max) else {
        unreachable!("the fixture mover carries a TuMax");
    };
    set_tu(app, mover, full);
}

/// Put the enter-provoked exchange behind a turn boundary and check the fixture survived it.
pub(crate) fn settle_the_enter_exchange(app: &mut App, reactors: &[Entity], mover: Entity) {
    cycle_back_to_player_turn(app);
    for &reactor in reactors {
        assert_eq!(
            used_of(app, reactor),
            Some(0),
            "fixture precondition: the turn boundary reset this reactor's per-turn cap",
        );
        assert!(
            is_alive(app, reactor),
            "fixture precondition: the mounted reactor survived the enter exchange with a pool \
             that affords one interrupt, else survivability (not the layout) would gate the \
             control",
        );
        let cost = single_fire_cost(app, reactor);
        assert!(
            tu_of(app, reactor).is_some_and(|tu| tu >= cost),
            "fixture precondition: the mounted reactor survived the enter exchange with a pool \
             that affords one interrupt, else survivability (not the layout) would gate the \
             control",
        );
    }
    assert!(
        is_alive(app, mover),
        "fixture precondition: the mover survived the enter exchange and walked, else the mover \
         (not the layout) would gate the control",
    );
    ready_the_mover(app, mover);
}

/// Where the mover stands as the measured move begins — its own turn may have moved it.
pub(crate) fn walking_from(app: &App, mover: Entity) -> CellLevel {
    let Some(at) = pos_of(app, mover) else {
        unreachable!("the fixture mover stands somewhere");
    };
    at
}

/// Fail unless the mover walked off `from`.
pub(crate) fn assert_the_mover_walked(app: &App, mover: Entity, from: CellLevel) {
    assert_ne!(
        pos_of(app, mover),
        Some(from),
        "fixture precondition: the mover survived the enter exchange and walked off {from:?}, \
         else the mover (not the layout) would gate the control",
    );
}

pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, position)| ***position == at)
        .map(|(entity, _)| entity)
}

pub(crate) fn set_tu(app: &mut App, entity: Entity, value: u8) {
    let Some(mut tu) = app.world_mut().get_mut::<Tu>(entity) else {
        unreachable!("the fixture ganger carries a Tu pool");
    };
    *tu = Tu::new(value);
}

pub(crate) fn suppress(app: &mut App, entity: Entity, from: CellLevel) {
    app.world_mut()
        .entity_mut(entity)
        .insert(Suppressed::new(SuppressorCell::new(from)));
}

pub(crate) fn wielded_single_mode(
    app: &mut App,
    shooter: Entity,
) -> gdtf_battle_sim::weapon::FireModeSpec {
    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    let mode = wielded
        .into_iter()
        .find_map(|entity| world.get::<FireMode>(entity).map(FireMode::single));
    let Some(mode) = mode else {
        unreachable!("the shooter wields a ranged weapon carrying a FireMode");
    };
    mode
}

pub(crate) fn single_fire_cost(app: &mut App, shooter: Entity) -> u8 {
    let mode = wielded_single_mode(app, shooter);
    let world = app.world();
    let (Some(tu_max), Some(aiming), Some(tuning)) = (
        world.get::<TuMax>(shooter).copied(),
        world.get::<Aiming>(shooter).copied(),
        world.get_resource::<CombatTuning>(),
    ) else {
        unreachable!("the shooter carries TuMax + Aiming and the app carries CombatTuning");
    };
    *mode_tu_cost(&mode, &tu_max, &aiming, tuning)
}

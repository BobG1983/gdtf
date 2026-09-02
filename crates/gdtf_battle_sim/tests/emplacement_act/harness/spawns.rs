//! Putting gangers and emplacements on the map, and finding them again.

use bevy::{app::App, asset::uuid::Uuid, prelude::Entity};
use gdtf_battle_sim::{
    acts::EnterEmplacementRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    ganger::{Cool, Direction, Facing, Grit, Position, Speed, Strength, Toughness},
    metric::CellLevel,
    prelude::{Faction, Stance, StanceKind},
    situation::GangerSpawn,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
            TerrainUuid, TerrainViews,
        },
        emplacement::EmplacementState,
        entity::TerrainCell,
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
    test_support::{GangerSpawnBuilder, TEST_MOUNTED_WEAPON_KEY},
    weapon::WeaponName,
};

use super::{
    PLAYER, SHOOTER_KEY,
    reads::{occupant, state},
    step,
    weapons::OWN_KEY,
};

/// A standing player ganger carrying the suite's own gun.
pub(crate) fn player_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    player_spawn(at, facing, Stance::new(StanceKind::Standing), OWN_KEY)
}

/// The same ganger crouched, so a round aimed high clears it and strikes what it mans.
pub(crate) fn crouching_player_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    player_spawn(at, facing, Stance::new(StanceKind::Crouching), OWN_KEY)
}

/// A standing player ganger carrying the tight gun, for a shot that must land on a cell.
pub(crate) fn shooter_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    player_spawn(at, facing, Stance::new(StanceKind::Standing), SHOOTER_KEY)
}

fn player_spawn(at: CellLevel, facing: Direction, stance: Stance, weapon: &str) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(stance)
        .weapon(WeaponName::new(weapon.to_owned()))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

pub(crate) fn player_ganger(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)
}

/// The one player ganger standing on `at`, or a failure naming the cell and the count found.
pub(crate) fn player_on(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, faction, position)| ***faction == PLAYER && ***position == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one player ganger must stand on {at:?}, found {}",
        found.len(),
    );
    let [entity] = found[..] else {
        unreachable!("the count above is one");
    };
    entity
}

/// The one emplacement seeded at `at`, or a failure naming the cell.
pub(crate) fn seated_emplacement(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TerrainCell, &EmplacementState)>();
    let matches: Vec<Entity> = query
        .iter(world)
        .filter(|(_, cell, _)| ***cell == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exactly one emplacement must be seeded at {at:?}, found {}",
        matches.len(),
    );
    let [entity] = matches[..] else {
        unreachable!("the count above is one");
    };
    entity
}

/// Enter the seat, and fail unless the enter actually manned it.
pub(crate) fn mount(app: &mut App, actor: Entity, emplacement: Entity) {
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(app, 3);
    assert_eq!(
        state(app, emplacement),
        Some(EmplacementState::Occupied),
        "PRECONDITION: the enter must man the seat, or nothing below is about a mounted ganger",
    );
    assert_eq!(
        occupant(app, emplacement),
        Some(actor),
        "PRECONDITION: the seat must name this actor as its occupant — a failed enter leaves \
         MountedBy absent and every assertion below passes while proving nothing",
    );
}

/// The one-sided emplacement def, whose single authored side makes a constrained route visible.
pub(crate) const ONE_SIDED: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_1186_0001));

/// The only side [`one_sided_emplacement`] authors, before a placed facing turns it.
pub(crate) const AUTHORED_SIDE: TerrainFacing = TerrainFacing::North;

/// The facing that piece is seeded at, which turns its one authored side to East.
pub(crate) const PLACED_FACING: TerrainFacing = TerrainFacing::East;

/// An emplacement def naming exactly one entry side, so one rotated entry cell reaches it.
pub(crate) fn one_sided_emplacement() -> TerrainDef {
    TerrainDef {
        key:            ONE_SIDED,
        display_name:   TerrainDisplayName::new("One-Sided Mount".to_owned()),
        sim_kind:       TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            entry_sides:      vec![AUTHORED_SIDE],
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("emplacement".to_owned()),
        },
        views:          TerrainViews::new(Vec::new()),
        tags:           Vec::new(),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

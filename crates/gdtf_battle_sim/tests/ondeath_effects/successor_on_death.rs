//! The cell's on-death entry belongs to the successor once the piece it named is gone.

use bevy::{app::App, asset::uuid::Uuid, prelude::Entity};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    effects::{
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
            ImmuneArmorTypes,
        },
        on_death::OnDeathEffect,
    },
    ganger::Direction,
    metric::{Cell, CellLevel, Level},
    situation::CoverSpawn,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
    test_support::{SituationBuilder, field_turns, single_mode, test_terrain_registry},
    weapon::DamageType,
};

use super::harness::*;

/// A barrel whose own death burns, and which leaves [`ASH_BARREL`] behind.
const SMOKING_BARREL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0547_1308_0001));
/// The successor barrel, whose own death chokes.
const ASH_BARREL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0547_1308_0002));
/// A barrel whose own death burns, and which leaves [`INERT_BARREL`] behind.
const PLAIN_BARREL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0547_1308_0003));
/// The successor barrel that authors no on-death at all.
const INERT_BARREL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0547_1308_0004));

/// The damage the burning field deals, which tells it from the choking one.
const BURNING_DAMAGE: u16 = 3;
/// The damage the choking field deals, which tells it from the burning one.
const CHOKING_DAMAGE: u16 = 9;

fn burning() -> FieldKey {
    FieldKey::new("burning".to_owned())
}

fn choking() -> FieldKey {
    FieldKey::new("choking".to_owned())
}

fn barrel_cell() -> CellLevel {
    ground(8, 5)
}

fn barrel_def(key: TerrainUuid, field: Option<FieldKey>, leaves: LeavesBehind) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Successor Barrel".to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(1),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags: Vec::new(),
        on_death: field
            .map(|field| OnDeathEffect::LeaveField { field })
            .into_iter()
            .collect(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: leaves,
    }
}

fn successor_terrain_registry() -> TerrainDefRegistry {
    let mut registry = test_terrain_registry();
    registry.insert(
        SMOKING_BARREL,
        barrel_def(
            SMOKING_BARREL,
            Some(burning()),
            LeavesBehind::Piece(ASH_BARREL),
        ),
    );
    registry.insert(
        ASH_BARREL,
        barrel_def(ASH_BARREL, Some(choking()), LeavesBehind::Nothing),
    );
    registry.insert(
        PLAIN_BARREL,
        barrel_def(
            PLAIN_BARREL,
            Some(burning()),
            LeavesBehind::Piece(INERT_BARREL),
        ),
    );
    registry.insert(
        INERT_BARREL,
        barrel_def(INERT_BARREL, None, LeavesBehind::Nothing),
    );
    registry
}

fn two_field_registry() -> FieldDefRegistry {
    FieldDefRegistry::new([
        (burning(), field_def(BURNING_DAMAGE)),
        (choking(), field_def(CHOKING_DAMAGE)),
    ])
}

fn field_def(damage: u16) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(damage),
        DamageType::Plasma,
        ImmuneArmorTypes::default(),
        FieldDuration::Turns(field_turns(2)),
    )
}

/// The damage of whatever field stands at the barrel cell, which names which one fanned.
fn field_damage_at_barrel(app: &App) -> Option<u16> {
    app.world()
        .get_resource::<FieldRegistry>()
        .and_then(|registry| registry.field_at(&barrel_cell()))
        .map(|placed| *placed.def().damage)
}

fn clear_fields(app: &mut App) {
    app.world_mut().insert_resource(FieldRegistry::new());
}

/// A live battle with the named barrel in front of a shooter that fells it in one round.
fn battle_with_barrel(seed: u64, piece: TerrainUuid) -> (App, Entity) {
    let (mut app, seed) = battle_app(seed, false);
    app.insert_resource(successor_terrain_registry());
    app.insert_resource(two_field_registry());

    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .with_scatter(CoverSpawn::new(
            barrel_cell(),
            piece,
            TerrainFacing::default(),
        ))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };
    (app, shooter_e)
}

fn fell_the_barrel(app: &mut App, shooter: Entity) {
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(barrel_cell().x, barrel_cell().y),
        Level::new(0),
    ));
    step(app, 4);
}

#[test]
fn the_successor_def_owns_the_cells_on_death_after_the_first_piece_falls() {
    let (mut app, shooter) = battle_with_barrel(0x5547_1308, SMOKING_BARREL);

    fell_the_barrel(&mut app, shooter);
    assert_eq!(
        field_damage_at_barrel(&app),
        Some(BURNING_DAMAGE),
        "the destroyed def's own effect fires, so the rewrite must not swallow it",
    );

    clear_fields(&mut app);
    fell_the_barrel(&mut app, shooter);
    assert_eq!(
        field_damage_at_barrel(&app),
        Some(CHOKING_DAMAGE),
        "felling the successor fires the successor def's effect, not the first def's",
    );
}

#[test]
fn a_successor_with_no_on_death_leaves_the_cell_with_nothing_to_fire() {
    let (mut app, shooter) = battle_with_barrel(0x5547_1309, PLAIN_BARREL);

    fell_the_barrel(&mut app, shooter);
    assert_eq!(
        field_damage_at_barrel(&app),
        Some(BURNING_DAMAGE),
        "the destroyed def's own effect fires before the rewrite lands",
    );

    clear_fields(&mut app);
    fell_the_barrel(&mut app, shooter);
    assert_eq!(
        field_damage_at_barrel(&app),
        None,
        "the successor authors no on-death, so the rewrite removed the cell's entry and \
         nothing fans the second time",
    );
}

//! Defs, specs and catalogs for the tests that read a whole on-death list.

use bevy::app::App;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    effects::{
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
            ImmuneArmorTypes,
        },
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    metric::CellLevel,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        piece::TerrainGraphicKey,
    },
    test_support::{TEST_WEAPON_KEY, field_turns},
    weapon::{BlastRadius, DamageType, HitType, WeaponName, WeaponRegistry, WeaponSpec},
};

use super::harness::explode_weapon_spec;

/// A one-HP cover piece whose on-death list each test supplies.
pub(crate) const ORDERED_COVER: TerrainUuid =
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0547_0547_0003));

/// The field key both a cover def and a weapon spec leave first.
pub(crate) fn burning() -> FieldKey {
    FieldKey::new("burning".to_owned())
}

/// The second field key, told apart from `burning` by its damage channel.
pub(crate) fn toxic() -> FieldKey {
    FieldKey::new("toxic".to_owned())
}

/// The `burning` def: a Plasma channel field.
pub(crate) fn burning_def() -> FieldDef {
    FieldDef::new(
        FieldDamage::new(3),
        DamageType::Plasma,
        ImmuneArmorTypes::default(),
        FieldDuration::Turns(field_turns(2)),
    )
}

/// The `toxic` def: the same shape on a different damage channel.
pub(crate) fn toxic_def() -> FieldDef {
    FieldDef::new(
        FieldDamage::new(3),
        DamageType::Kinetic,
        ImmuneArmorTypes::default(),
        FieldDuration::Turns(field_turns(2)),
    )
}

/// The catalog both keys resolve through.
pub(crate) fn two_field_def_registry() -> FieldDefRegistry {
    FieldDefRegistry::new([(burning(), burning_def()), (toxic(), toxic_def())])
}

/// Insert the two field defs, without the barrel terrain `battle_app` pairs them with.
pub(crate) fn insert_field_defs(app: &mut App) {
    app.insert_resource(two_field_def_registry());
}

/// A blast that damages every neighbour of the death cell.
pub(crate) const fn explode_effect() -> OnDeathEffect {
    OnDeathEffect::Explode {
        hit_type:    HitType::Blast {
            radius: BlastRadius::new(1),
        },
        damage:      ExplodeDamage::new(8),
        damage_type: DamageType::Blast,
    }
}

/// Leave the named field at the death cell.
pub(crate) const fn leave_field_effect(field: FieldKey) -> OnDeathEffect {
    OnDeathEffect::LeaveField { field }
}

/// A terrain catalog whose one cover def authors the supplied on-death list.
pub(crate) fn ordered_cover_registry(on_death: Vec<OnDeathEffect>) -> TerrainDefRegistry {
    let mut base = gdtf_battle_sim::test_support::test_terrain_registry();
    base.insert(
        ORDERED_COVER,
        TerrainDef {
            key: ORDERED_COVER,
            display_name: TerrainDisplayName::new("Ordered Cover".to_owned()),
            sim_kind: TerrainSimKind::Cover {
                hp:               CoverHp::new(1),
                armor_protection: ArmorProtection::new(0),
                armor_hardness:   ArmorHardness::new(0),
                height_band:      HeightBand::Low,
            },
            presenter_kind: TerrainPresenterKind::Cover {
                graphic_name: TerrainGraphicKey::new("cover".to_owned()),
            },
            views: TerrainViews::new(Vec::new()),
            tags: Vec::new(),
            on_death,
            blocks_pathing: None,
            blocks_los: None,
            leaves_behind: LeavesBehind::Nothing,
        },
    );
    base
}

/// A weapon catalog whose test weapon authors the supplied on-death list.
pub(crate) fn ordered_weapon_registry(on_death: Vec<OnDeathEffect>) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        WeaponSpec {
            on_death,
            ..explode_weapon_spec()
        },
    )])
}

/// The field def standing at a cell, if any.
pub(crate) fn field_def_at(app: &App, at: CellLevel) -> Option<FieldDef> {
    app.world()
        .get_resource::<FieldRegistry>()
        .and_then(|fields| fields.field_at(&at).map(|placed| placed.def().clone()))
}

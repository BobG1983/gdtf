use super::support::*;
use crate::{
    effects::fields::{FieldDamage, FieldDef, FieldDuration, FieldKey, ImmuneArmorTypes},
    terrain::entity::TerrainIndexKey,
    test_support::field_turns,
};

const BYSTANDER_HP: u16 = 10;

fn burning() -> FieldKey {
    FieldKey::new("burning".to_owned())
}

#[test]
fn a_cover_death_and_a_slab_death_at_one_cell_each_fan_their_own_effect() {
    let mut app = resolver_app();
    let at = ground(5, 5);

    let mut defs = FieldDefRegistry::default();
    defs.insert(
        burning(),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(field_turns(2)),
        ),
    );
    app.world_mut().insert_resource(defs);

    let mut terrain = TerrainOnDeathRegistry::default();
    terrain.insert(
        TerrainIndexKey::Slab(at),
        OnDeathEffect::LeaveField { field: burning() },
    );
    terrain.insert(
        TerrainIndexKey::Cover(at),
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(5),
            damage_type: DamageType::Blast,
        },
    );
    app.world_mut().insert_resource(terrain);

    let bystander = app
        .world_mut()
        .spawn((
            Hp::new(BYSTANDER_HP),
            LifeState::Alive,
            Position::new(ground(6, 5)),
        ))
        .id();
    grid_with_occupant(&mut app, ground(6, 5), bystander);

    assert!(
        app.world()
            .get_resource::<FieldRegistry>()
            .is_some_and(|r| r.field_at(&at).is_none()),
        "no field may stand at the death cell before the two deaths are written",
    );

    app.world_mut().write_message(OnDeathOccurred::slab(at));
    app.world_mut().write_message(OnDeathOccurred::cover(at));
    app.update();

    assert!(
        app.world()
            .get_resource::<FieldRegistry>()
            .is_some_and(|r| r.field_at(&at).is_some()),
        "the SLAB death's LeaveField did not fan — no field at the death cell",
    );
    assert!(
        hp_of(&app, bystander) < BYSTANDER_HP,
        "the COVER death's Explode did not fan — the bystander still holds {BYSTANDER_HP} HP",
    );
}

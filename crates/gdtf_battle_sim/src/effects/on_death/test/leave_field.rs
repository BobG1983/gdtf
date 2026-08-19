use super::support::*;
use crate::{
    effects::fields::{FieldDamage, FieldDef, FieldDuration, FieldKey, ImmuneArmorTypes},
    terrain::entity::TerrainIndexKey,
    test_support::field_turns,
};

#[test]
fn leave_field_spawns_the_referenced_field_at_the_death_cell() {
    let mut app = resolver_app();
    let mut defs = FieldDefRegistry::default();
    defs.insert(
        FieldKey::new("burning".to_owned()),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(field_turns(2)),
        ),
    );
    app.world_mut().insert_resource(defs);

    let mut cover = TerrainOnDeathRegistry::default();
    cover.insert(
        TerrainIndexKey::Cover(ground(7, 8)),
        OnDeathEffect::LeaveField {
            field: FieldKey::new("burning".to_owned()),
        },
    );
    app.world_mut().insert_resource(cover);

    app.world_mut()
        .write_message(OnDeathOccurred::cover(ground(7, 8)));
    app.update();

    let registry = app.world().get_resource::<FieldRegistry>();
    assert!(
        registry.is_some_and(|r| r.field_at(&ground(7, 8)).is_some()),
        "the referenced field was spawned at the death cell"
    );
}

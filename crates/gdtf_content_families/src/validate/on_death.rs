//! On-death `LeaveField` edges from terrain defs and ranged weapon specs.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily, FindingReferrer,
    FindingTarget, ReferenceField, ReferenceKeyScheme, ReferringRecord,
};
use gdtf_battle_sim::{
    effects::{
        fields::{FieldDefRegistry, FieldKey},
        on_death::OnDeathEffect,
    },
    terrain::def::TerrainDefRegistry,
    weapon::WeaponRegistry,
};

// Every field key an authored on-death list places.
fn left_fields(effects: &[OnDeathEffect]) -> impl Iterator<Item = &FieldKey> {
    effects.iter().filter_map(|effect| match effect {
        OnDeathEffect::LeaveField { field } => Some(field),
        OnDeathEffect::Explode { .. } => None,
    })
}

/// Record dangling `on_death` `LeaveField` → field def references.
pub fn check_on_death_field_refs(
    terrain: Res<TerrainDefRegistry>,
    weapons: Res<WeaponRegistry>,
    fields: Res<FieldDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        for field in left_fields(&def.on_death) {
            if fields.def(field).is_none() {
                report.record(dangling(
                    FindingReferrer::new(format!(
                        "terrain def `{}` ({}) on_death LeaveField",
                        *def.display_name, **key,
                    )),
                    FindingFamily::new("TerrainDefRegistry".to_owned()),
                    ContentMemberKey::new((**key).to_string()),
                    field,
                ));
            }
        }
    }
    for (name, spec) in weapons.iter() {
        for field in left_fields(&spec.on_death) {
            if fields.def(field).is_none() {
                report.record(dangling(
                    FindingReferrer::new(format!("weapon `{}` on_death LeaveField", **name)),
                    FindingFamily::new("WeaponRegistry".to_owned()),
                    ContentMemberKey::new((**name).clone()),
                    field,
                ));
            }
        }
    }
}

// One finding, naming the record a drop would have to rewrite.
fn dangling(
    referrer: FindingReferrer,
    family: FindingFamily,
    key: ContentMemberKey,
    field: &FieldKey,
) -> ContentFinding {
    ContentFinding::DanglingRef {
        referrer,
        referring_record: ReferringRecord::new(
            family,
            key,
            ReferenceField::new("on_death[].LeaveField.field".to_owned()),
        ),
        target: FindingTarget::new((**field).clone()),
        family: FindingFamily::new("FieldDefRegistry".to_owned()),
        scheme: ReferenceKeyScheme::FileStem,
    }
}

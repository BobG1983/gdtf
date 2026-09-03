//! Cross-check prefab theme and placement UUIDs against the theme and terrain registries.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily, FindingReferrer,
    FindingTarget, ReferenceField, ReferenceKeyScheme, ReferringRecord,
};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

/// Record dangling prefab → theme UUID and prefab → placed terrain UUID references.
pub fn check_prefab_refs(
    prefabs: Res<PrefabRegistry>,
    themes: Res<UuidThemeRegistry>,
    terrain: Res<TerrainDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for prefab in prefabs.iter() {
        let spec = prefab.spec();
        let record = |field: &str| {
            ReferringRecord::new(
                FindingFamily::new("PrefabRegistry".to_owned()),
                ContentMemberKey::new((**prefab.name()).clone()),
                ReferenceField::new(field.to_owned()),
            )
        };
        if themes.def(&spec.theme).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer:         FindingReferrer::new(format!(
                    "prefab `{}` theme",
                    **prefab.name()
                )),
                referring_record: record("theme"),
                target:           FindingTarget::new(spec.theme.to_string()),
                family:           FindingFamily::new("UuidThemeRegistry".to_owned()),
                scheme:           ReferenceKeyScheme::Uuid,
            });
        }
        let mut seen: Vec<TerrainUuid> = Vec::new();
        for placement in &spec.placements {
            if seen.contains(&placement.piece) {
                continue;
            }
            seen.push(placement.piece);
            if terrain.def(&placement.piece).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer:         FindingReferrer::new(format!(
                        "prefab `{}` placements",
                        **prefab.name(),
                    )),
                    referring_record: record("placements[].piece"),
                    target:           FindingTarget::new(placement.piece.to_string()),
                    family:           FindingFamily::new("TerrainDefRegistry".to_owned()),
                    scheme:           ReferenceKeyScheme::Uuid,
                });
            }
        }
    }
}

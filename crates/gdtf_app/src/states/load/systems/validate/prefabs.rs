use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
};

pub(super) fn check_prefab_refs(
    prefabs: Res<PrefabRegistry>,
    themes: Res<UuidThemeRegistry>,
    terrain: Res<TerrainDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for prefab in prefabs.iter() {
        let spec = prefab.spec();
        if themes.def(&spec.theme).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!("prefab `{}` theme", **prefab.name())),
                target:   FindingTarget::new(spec.theme.to_string()),
                family:   FindingFamily::new("UuidThemeRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::Uuid,
            });
        }
        let mut seen: Vec<gdtf_battle_sim::terrain::def::TerrainUuid> = Vec::new();
        for placement in &spec.placements {
            if seen.contains(&placement.piece) {
                continue;
            }
            seen.push(placement.piece);
            if terrain.def(&placement.piece).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: FindingReferrer::new(format!(
                        "prefab `{}` placements",
                        **prefab.name(),
                    )),
                    target:   FindingTarget::new(placement.piece.to_string()),
                    family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
                    scheme:   ReferenceKeyScheme::Uuid,
                });
            }
        }
    }
}

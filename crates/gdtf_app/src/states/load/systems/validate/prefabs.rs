//! GTW-582: the **prefab fragments'** outbound reference edges — each
//! fragment's theme UUID (the prefab → theme agreement: a fragment bucketed
//! under a theme no registry holds can never pour that theme's floor) and every
//! placed piece's terrain-def UUID (the fail-open pour's root cause, C3(d)).

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
};

/// `Check`: every prefab fragment's `theme` UUID resolves in the
/// [`UuidThemeRegistry`], and every placed piece's terrain UUID resolves in the
/// [`TerrainDefRegistry`] (deduplicated per fragment — a repeated wall piece is
/// ONE finding).
///
/// Plain `Res` params are safe here: the `Check` set's window condition
/// verified them present (bevy-traps #1, guarded once at the set).
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

//! Check every terrain def provides art for the views its own kind and tags owe.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{ContentFinding, ContentIntegrityReport, FindingReferrer, FindingTarget};
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, owed_views};

/// Record one finding per terrain def that answers fewer views than it owes.
pub fn check_terrain_view_coverage(
    terrain: Res<TerrainDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        let missing: Vec<FindingTarget> = owed_views(def)
            .iter()
            .filter(|view| def.views.sprite(**view).is_none())
            .map(|view| FindingTarget::new(format!("{view:?}")))
            .collect();
        if missing.is_empty() {
            continue;
        }
        report.record(ContentFinding::MissingViews {
            referrer: FindingReferrer::new(format!(
                "terrain def `{}` ({})",
                *def.display_name, **key,
            )),
            views:    missing,
        });
    }
}

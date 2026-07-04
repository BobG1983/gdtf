//! GTW-582 C3(c): the **injury weightings'** reference edges — every weighting
//! row's injury key. The build-time behavior is untouched (an unknown key is
//! still `warn!`-skipped in `build_tables`, so the roll never crashes); this
//! check ALSO reports each dangling row through the unified pass, so the
//! mistake lands on the ONE consolidated report.

use bevy::prelude::{Assets, Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme, RonAsset,
};
use gdtf_battle_sim::{
    injuries::{InjuryRegistry, InjuryWeighting},
    severity::Severity,
};

/// `Check`: every AUTHORED weighting row's injury key resolves in the
/// [`InjuryRegistry`]. Walks the loaded `RonAsset<InjuryWeighting>` collection
/// (the authored rows — the built [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables)
/// only holds the SURVIVING rows, so it cannot be the source here).
///
/// The collection is `Option<Res<…>>`: it exists only when the RON loader
/// registered (a real `AssetServer` app); a `MinimalPlugins` harness has no
/// authored weightings to check, so the check no-ops (bevy-traps #1).
pub(super) fn check_injury_weighting_refs(
    weightings: Option<Res<Assets<RonAsset<InjuryWeighting>>>>,
    injuries: Res<InjuryRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let Some(weightings) = weightings else {
        return;
    };
    for (_id, weighting) in weightings.iter() {
        for (severity, rows) in [
            (Severity::Minor, &weighting.minor),
            (Severity::Major, &weighting.major),
            (Severity::Critical, &weighting.critical),
        ] {
            for row in rows {
                if !injuries.contains(&row.injury) {
                    report.record(ContentFinding::DanglingRef {
                        referrer: FindingReferrer::new(format!(
                            "injury weighting `{:?}` ({severity:?} bucket)",
                            weighting.category,
                        )),
                        target:   FindingTarget::new((*row.injury).clone()),
                        family:   FindingFamily::new("InjuryRegistry".to_owned()),
                        scheme:   ReferenceKeyScheme::FileStem,
                    });
                }
            }
        }
    }
}

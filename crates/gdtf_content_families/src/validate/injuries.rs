//! Cross-check injury weighting keys against the injury registry.

use bevy::prelude::{Assets, Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily, FindingReferrer,
    FindingTarget, ReferenceField, ReferenceKeyScheme, ReferringRecord, RonAsset,
};
use gdtf_battle_sim::{
    injuries::{InjuryRegistry, InjuryWeighting},
    severity::Severity,
};

/// Record dangling weighting → injury name references.
pub fn check_injury_weighting_refs(
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
                        referrer:         FindingReferrer::new(format!(
                            "injury weighting `{:?}` ({:?} {severity:?} bucket)",
                            weighting.category, weighting.context,
                        )),
                        referring_record: ReferringRecord::new(
                            FindingFamily::new("InjuryWeighting".to_owned()),
                            ContentMemberKey::new(format!(
                                "{:?}/{:?}",
                                weighting.category, weighting.context,
                            )),
                            ReferenceField::new(format!("{severity:?}[].injury")),
                        ),
                        target:           FindingTarget::new((*row.injury).clone()),
                        family:           FindingFamily::new("InjuryRegistry".to_owned()),
                        scheme:           ReferenceKeyScheme::FileStem,
                    });
                }
            }
        }
    }
}

use std::path::Path;

use bevy::prelude::*;
use gdtf_assets::{ContentFinding, ContentIntegrityReport, ReferenceKeyScheme};

use crate::content_shared::app::editor_app_with_asset_root;

pub(crate) const DANGLING_DEFAULT_FLOOR: &str = "00000000-0000-0000-0000-063000000001";

pub(crate) const DANGLING_PALETTE: &str = "00000000-0000-0000-0000-063000000002";

pub(crate) fn editor_app_on_fixture_root() -> App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate_root");
    editor_app_with_asset_root(&root)
}

pub(crate) fn has_malformed(report: &ContentIntegrityReport, stem: &str) -> bool {
    report.findings().iter().any(|finding| {
        matches!(
            finding,
            ContentFinding::MalformedFile { path, .. } if path.contains(stem)
        )
    })
}

/// The referring record of the first dangling finding matching family, target and scheme,
/// as its family name, key and field name.
pub(crate) fn dangling_ref_record(
    report: &ContentIntegrityReport,
    family: &str,
    target: &str,
    scheme: ReferenceKeyScheme,
) -> Option<(String, String, String)> {
    report.findings().iter().find_map(|finding| match finding {
        ContentFinding::DanglingRef {
            referring_record,
            target: found_target,
            family: found_family,
            scheme: found_scheme,
            ..
        } if **found_target == *target && **found_family == *family && *found_scheme == scheme => {
            Some((
                (*referring_record.family).clone(),
                (*referring_record.key).clone(),
                (*referring_record.field).clone(),
            ))
        }
        _ => None,
    })
}

/// The view lists of every `MissingViews` finding raised against the def this names.
pub(crate) fn missing_views(
    report: &ContentIntegrityReport,
    referrer_hint: &str,
) -> Vec<Vec<String>> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match finding {
            ContentFinding::MissingViews { referrer, views }
                if referrer.contains(referrer_hint) =>
            {
                Some(views.iter().map(|view| (**view).clone()).collect())
            }
            _ => None,
        })
        .collect()
}

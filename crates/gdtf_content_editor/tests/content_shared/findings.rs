//! Reading dangling-reference findings out of a published integrity report.

use gdtf_assets::{ContentFinding, ContentIntegrityReport, ReferenceKeyScheme};

/// Whether the report raises a dangling reference for this family, target and scheme.
pub(crate) fn has_dangling_ref(
    report: &ContentIntegrityReport,
    family: &str,
    target: &str,
    scheme: ReferenceKeyScheme,
) -> bool {
    dangling_ref_referrer(report, family, target, scheme).is_some()
}

/// The referrer text of the first dangling finding matching family, target and scheme.
pub(crate) fn dangling_ref_referrer(
    report: &ContentIntegrityReport,
    family: &str,
    target: &str,
    scheme: ReferenceKeyScheme,
) -> Option<String> {
    report.findings().iter().find_map(|finding| match finding {
        ContentFinding::DanglingRef {
            referrer,
            referring_record: _,
            target: found_target,
            family: found_family,
            scheme: found_scheme,
        } if **found_target == *target && **found_family == *family && *found_scheme == scheme => {
            Some((**referrer).clone())
        }
        _ => None,
    })
}

//! Individual integrity findings (dangling refs, malformed files, degraded fallback).

use bevy::prelude::Deref;

use crate::family::def::ContentMemberKey;

/// Where a bad reference was authored (file / key context).
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingReferrer(String);

impl FindingReferrer {
    /// Wrap a referrer string.
    #[must_use]
    pub const fn new(referrer: String) -> Self {
        Self(referrer)
    }
}

/// Target key that failed to resolve.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingTarget(String);

impl FindingTarget {
    /// Wrap a target string.
    #[must_use]
    pub const fn new(target: String) -> Self {
        Self(target)
    }
}

/// Registry / family name in a finding.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingFamily(String);

impl FindingFamily {
    /// Wrap a family label.
    #[must_use]
    pub const fn new(family: String) -> Self {
        Self(family)
    }
}

/// Name of the field on the referring record that holds the reference.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ReferenceField(String);

impl ReferenceField {
    /// Wrap a field name.
    #[must_use]
    pub const fn new(field: String) -> Self {
        Self(field)
    }
}

/// The record that holds a reference, named so a rewrite can find its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferringRecord {
    /// Family the referring record belongs to.
    pub family: FindingFamily,
    /// Key the referring record was inserted under.
    pub key:    ContentMemberKey,
    /// Field on that record holding the reference.
    pub field:  ReferenceField,
}

impl ReferringRecord {
    /// Name a referring record by family, key and field.
    #[must_use]
    pub const fn new(family: FindingFamily, key: ContentMemberKey, field: ReferenceField) -> Self {
        Self { family, key, field }
    }
}

/// Free-form detail text.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingDetail(String);

impl FindingDetail {
    /// Wrap detail text.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }
}

/// How a reference key is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKeyScheme {
    /// Key is a file stem.
    FileStem,
    /// Key is a display name.
    DisplayName,
    /// Key is a UUID.
    Uuid,
}

impl core::fmt::Display for ReferenceKeyScheme {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let label = match self {
            Self::FileStem => "file-stem key",
            Self::DisplayName => "display-name key",
            Self::Uuid => "UUID key",
        };
        f.write_str(label)
    }
}

/// One content integrity finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentFinding {
    /// Reference points at a missing member.
    DanglingRef {
        /// Authoring context.
        referrer:         FindingReferrer,
        /// The record holding the reference, by family, key and field.
        referring_record: ReferringRecord,
        /// Missing key.
        target:           FindingTarget,
        /// Registry family.
        family:           FindingFamily,
        /// Key scheme used.
        scheme:           ReferenceKeyScheme,
    },
    /// A single file failed to load; siblings may still be ok.
    MalformedFile {
        /// Path of the bad file.
        path:   FindingReferrer,
        /// Registry family.
        family: FindingFamily,
        /// Load error detail.
        detail: FindingDetail,
    },
    /// A def provides no art for views it owes.
    MissingViews {
        /// Authoring context.
        referrer: FindingReferrer,
        /// The views the def names no art for.
        views:    Vec<FindingTarget>,
    },
    /// Host fell back to a degraded default.
    DegradedFallback {
        /// Context of the fallback.
        context: FindingReferrer,
        /// Why fallback was used.
        detail:  FindingDetail,
    },
}

impl core::fmt::Display for ContentFinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DanglingRef {
                referrer,
                referring_record: _,
                target,
                family,
                scheme,
            } => write!(
                f,
                "dangling reference: {} names `{}` ({scheme}), not found in {}",
                &***referrer, &***target, &***family,
            ),
            Self::MalformedFile {
                path,
                family,
                detail,
            } => write!(
                f,
                "malformed file: `{}` failed to load into {} ({}); its well-formed \
                 siblings were salvaged per-file",
                &***path, &***family, &***detail,
            ),
            Self::MissingViews { referrer, views } => {
                let named: Vec<&str> = views.iter().map(|view| &***view).collect();
                write!(f, "{} is missing {}", &***referrer, named.join(", "))
            }
            Self::DegradedFallback { context, detail } => {
                write!(f, "degraded fallback: {} — {}", &***context, &***detail)
            }
        }
    }
}

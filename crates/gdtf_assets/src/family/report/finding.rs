use bevy::prelude::Deref;

/// WHERE a dangling reference was authored — the referencing file / key context a
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingReferrer(String);

impl FindingReferrer {
        #[must_use]
    pub const fn new(referrer: String) -> Self {
        Self(referrer)
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingTarget(String);

impl FindingTarget {
        #[must_use]
    pub const fn new(target: String) -> Self {
        Self(target)
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingFamily(String);

impl FindingFamily {
        #[must_use]
    pub const fn new(family: String) -> Self {
        Self(family)
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingDetail(String);

impl FindingDetail {
        #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKeyScheme {
            FileStem,
            DisplayName,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentFinding {
        DanglingRef {
                referrer: FindingReferrer,
                target:   FindingTarget,
                family:   FindingFamily,
                scheme:   ReferenceKeyScheme,
    },
            MalformedFile {
                path:   FindingReferrer,
                family: FindingFamily,
                detail: FindingDetail,
    },
        DegradedFallback {
                context: FindingReferrer,
                detail:  FindingDetail,
    },
}

impl core::fmt::Display for ContentFinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DanglingRef {
                referrer,
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
            Self::DegradedFallback { context, detail } => {
                write!(f, "degraded fallback: {} — {}", &***context, &***detail)
            }
        }
    }
}

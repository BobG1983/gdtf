//! The GTW-582 finding VOCABULARY — the typed record one content-integrity
//! finding is (see the parent [`report`](super) module doc for the contract).

use bevy::prelude::Deref;

/// WHERE a dangling reference was authored — the referencing file / key context a
/// finding prints (e.g. `content/situations/skirmish.ron: ganger 'Vex 1'`).
///
/// A named newtype over the label `String` (no-bare-types rule; private inner per
/// rule 5), read through [`Deref`].
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingReferrer(String);

impl FindingReferrer {
    /// Wrap a referencing file / key context label.
    #[must_use]
    pub const fn new(referrer: String) -> Self {
        Self(referrer)
    }
}

/// The dangling TARGET key a reference names but no registry resolves.
///
/// A named newtype over the key `String` (no-bare-types rule; private inner per
/// rule 5), read through [`Deref`].
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingTarget(String);

impl FindingTarget {
    /// Wrap the unresolved target key.
    #[must_use]
    pub const fn new(target: String) -> Self {
        Self(target)
    }
}

/// The target FAMILY (registry) the dangling key should have resolved in — e.g.
/// `GangRegistry`, `TerrainDefRegistry`.
///
/// A named newtype over the family label `String` (no-bare-types rule; private
/// inner per rule 5), read through [`Deref`].
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingFamily(String);

impl FindingFamily {
    /// Wrap the target-family (registry) label.
    #[must_use]
    pub const fn new(family: String) -> Self {
        Self(family)
    }
}

/// Free-form finding detail — a load error's text, a fallback's description.
///
/// A named newtype over the detail `String` (no-bare-types rule; private inner
/// per rule 5), read through [`Deref`].
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct FindingDetail(String);

impl FindingDetail {
    /// Wrap the finding's detail text.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }
}

/// Which KEY SCHEME a dangling reference failed under — load-bearing on the gang
/// path, where TWO schemes coexist: a situation references a GANG by its file
/// STEM but a MEMBER by its roster display-name (GTW-582: findings must print
/// which scheme failed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKeyScheme {
    /// The key is an authored file's stem (`gang_0.gang.ron` → `gang_0`) — the
    /// gang / weapon / armor / melee / attachment / injury / field scheme.
    FileStem,
    /// The key is a human-facing display name authored INSIDE a file — the
    /// gang-roster MEMBER scheme (`member: "Vex 1"`).
    DisplayName,
    /// The key is a stable UUID authored inside a file — the terrain-def /
    /// theme scheme.
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

/// One load-time content-integrity finding — the record the unified reference
/// contract accumulates and prints (GTW-582 C2).
///
/// The three kinds mirror the three failure families the contract folds in:
/// a **dangling reference** (an authored key no registry resolves), a
/// **malformed file** (a member that failed to parse, whose well-formed siblings
/// were salvaged per-file — C4), and a **degraded fallback** (a last-resort
/// playable-but-degraded path that engaged — C5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentFinding {
    /// An authored reference names a key its target family does not hold.
    DanglingRef {
        /// The referencing file / key context.
        referrer: FindingReferrer,
        /// The unresolved target key.
        target:   FindingTarget,
        /// The registry family the key should have resolved in.
        family:   FindingFamily,
        /// The key scheme the reference failed under.
        scheme:   ReferenceKeyScheme,
    },
    /// A content member file failed to load/parse; its siblings were salvaged
    /// per-file (the C4 fail-closed-per-FILE contract).
    MalformedFile {
        /// The member file's asset path.
        path:   FindingReferrer,
        /// The registry family the file belongs to.
        family: FindingFamily,
        /// The load error's text.
        detail: FindingDetail,
    },
    /// A last-resort degraded fallback engaged (never silent — C5).
    DegradedFallback {
        /// The context the fallback engaged in.
        context: FindingReferrer,
        /// What degraded and to what.
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

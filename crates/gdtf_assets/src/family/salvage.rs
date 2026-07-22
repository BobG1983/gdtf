//! The GTW-582 **per-file folder salvage** — the fail-closed-per-FILE
//! replacement for the old fail-closed-per-FAMILY `Failed`-folder path (C4),
//! written ONCE here for the generic content-family machinery AND the bespoke resolvers.
//!
//! # Why a salvage pass exists
//!
//! Bevy's [`load_folder`](bevy::asset::AssetServer::load_folder) walk aborts the
//! WHOLE folder on the first member whose load errors (verified in
//! `bevy_asset-0.19.0/src/server/mod.rs::load_folder_internal`: a member
//! `AssetLoadError` returns `Err`, no [`LoadedFolder`] asset is ever inserted,
//! and the member handles collected so far are dropped). So ONE malformed `.ron`
//! used to fail its folder's `RecursiveDependencyLoadState` and the resolve then
//! published an EMPTY registry — every well-formed sibling vanished for one typo.
//!
//! The salvage pass re-walks the folder DIRECTORY through the asset server's own
//! source reader (the same reader `load_folder` walks), loads each matching
//! member INDIVIDUALLY (so one member's parse error can no longer poison its
//! siblings), and settles when every member has terminally Loaded or Failed:
//! loaded members fold into the registry, failed members surface as loud
//! [`MalformedFile`](crate::ContentFinding::MalformedFile) findings.
//!
//! A folder that cannot be enumerated at all (missing directory — the fixture
//! convention for an un-materialized content root) still fails closed to the
//! EMPTY registry exactly as before; salvage changes only the
//! malformed-MEMBER case.
//!
//! The directory enumeration runs `block_on` INSIDE the resolve system — a
//! deliberate, documented exception: it is local-FS metadata I/O, it runs at
//! most ONCE per family per run, and ONLY on the already-failed error path
//! (never in the happy Load flow).

use std::path::{Path, PathBuf};

use bevy::{
    asset::{
        AssetPath, AssetServer, Assets, Handle, LoadState,
        io::{AssetReaderError, AssetSourceId, ErasedAssetReader},
    },
    ecs::resource::Resource,
    prelude::Deref,
    reflect::TypePath,
    tasks::{block_on, futures_lite::StreamExt},
};

use crate::{
    asset::RonAsset,
    family::report::{
        ContentFinding, ContentIntegrityReport, FindingDetail, FindingFamily, FindingReferrer,
    },
};

/// A salvage member's asset path, relative to the asset source root — the key
/// callers derive a member's registry stem from.
///
/// A named newtype over the path `String` (no-bare-types rule; private inner per
/// rule 5), read through [`Deref`].
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct SalvageMemberPath(String);

impl SalvageMemberPath {
    /// Wrap a member's source-root-relative asset path.
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }
}

/// The content folder a salvage walked (source-root-relative) — the salvage's
/// logging context.
///
/// A named newtype over the folder `String` (no-bare-types rule; private inner
/// per rule 5), read through [`Deref`].
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct SalvageFolder(String);

impl SalvageFolder {
    /// Wrap a salvaged folder's source-root-relative path.
    #[must_use]
    pub const fn new(folder: String) -> Self {
        Self(folder)
    }
}

/// One salvaged folder member: its asset path (relative to the source root) and
/// the STRONG typed handle of its individual load.
#[derive(Debug)]
struct SalvageMember<T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// The member's asset path, relative to the asset source root.
    path:   SalvageMemberPath,
    /// The member's individually-loaded typed handle (held strong so the asset
    /// stays resident for the registry build, the redrive, and the file-watcher).
    handle: Handle<RonAsset<T>>,
}

/// The per-file salvage state of ONE content folder whose `load_folder` walk
/// failed — the member paths + strong typed handles the salvage loaded
/// individually (GTW-582 C4).
///
/// A marker-parameterized generic resource (the standing `Q4` rule-4 plumbing
/// carve-out; rule 5 binds — the inner list is PRIVATE, read through
/// [`poll_ron_folder_salvage`]). `T` is the member payload type, which is
/// family-unique (every family claims its own spec type), so two folders can
/// never collide on one salvage resource.
///
/// Inserted by the failing resolve (generic or bespoke) and NEVER removed: after
/// the salvage settles it keeps the member assets resident (the
/// [`ContentFolderHandle`](crate::ContentFolderHandle)'s strong-reference role,
/// which the failed folder load no longer plays) and serves as the redrive's
/// member enumeration source (the failed folder has no
/// [`LoadedFolder`](bevy::asset::LoadedFolder) asset
/// to re-walk).
#[derive(Resource, Debug)]
pub struct RonFolderSalvage<T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// The folder the salvage walked, for logging.
    folder:  SalvageFolder,
    /// The individually-loaded members.
    members: Vec<SalvageMember<T>>,
}

impl<T> RonFolderSalvage<T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// Whether the salvage found no matching member files at all (the folder is
    /// effectively empty for this family — the caller publishes the EMPTY
    /// registry immediately).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// The folder this salvage walked.
    #[must_use]
    pub const fn folder(&self) -> &SalvageFolder {
        &self.folder
    }
}

/// One SETTLED salvage member the caller folds into its registry: the member's
/// asset path plus its loaded payload. (No `Debug` — the payload `T` carries no
/// `Debug` bound, matching [`RonAsset`].)
pub struct SalvagedMember<'a, T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// The member's asset path, relative to the asset source root.
    pub path: &'a SalvageMemberPath,
    /// The member's loaded payload.
    pub spec: &'a RonAsset<T>,
}

/// One salvage member that terminally FAILED to load — the malformed file the
/// caller reports (loudly) and skips. Carries the report vocabulary directly
/// ([`FindingReferrer`] / [`FindingDetail`]) since its one consumer,
/// [`report_malformed_members`], turns it into a
/// [`MalformedFile`](ContentFinding::MalformedFile) finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedMember {
    /// The member's asset path, relative to the asset source root.
    pub path:  FindingReferrer,
    /// The load error's text.
    pub error: FindingDetail,
}

/// The outcome of polling a [`RonFolderSalvage`] this frame. (No `Debug` — the
/// loaded members' payload `T` carries no `Debug` bound.)
pub enum RonSalvagePoll<'a, T>
where
    T: TypePath + Send + Sync + 'static,
{
    /// At least one member is still loading — poll again next frame.
    Pending,
    /// Every member terminally Loaded or Failed — build the registry from
    /// `loaded` and report each `malformed` member.
    Settled {
        /// The members that loaded, in enumeration order.
        loaded:    Vec<SalvagedMember<'a, T>>,
        /// The members that terminally failed (the malformed files).
        malformed: Vec<MalformedMember>,
    },
}

/// Begin a per-file salvage of `folder`: enumerate its member files matching
/// `.{extension}` through the asset server's OWN source reader and start an
/// INDIVIDUAL typed load for each (GTW-582 C4).
///
/// The extension filter keeps a MIXED folder's other-family members (and stray
/// files) out of this family's salvage — the path-level twin of the generic
/// machinery's `TypeId` member filter.
///
/// # Errors
///
/// Returns the [`AssetReaderError`] when the folder cannot be enumerated at all
/// (missing directory / unreadable) — the caller then fails closed to the EMPTY
/// registry, exactly as the pre-GTW-582 `Failed` path did.
pub fn begin_ron_folder_salvage<T>(
    asset_server: &AssetServer,
    folder: &str,
    extension: &str,
) -> Result<RonFolderSalvage<T>, AssetReaderError>
where
    T: TypePath + Send + Sync + 'static,
{
    let paths = enumerate_member_paths(asset_server, folder, extension)?;
    let members = paths
        .into_iter()
        .map(|path| {
            let handle = asset_server.load::<RonAsset<T>>(AssetPath::parse(&path).into_owned());
            SalvageMember {
                path: SalvageMemberPath::new(path),
                handle,
            }
        })
        .collect();
    Ok(RonFolderSalvage {
        folder: SalvageFolder::new(folder.to_owned()),
        members,
    })
}

/// Poll a salvage's member loads: [`RonSalvagePoll::Pending`] while ANY member
/// is still in flight (or loaded but not yet in its `Assets` collection — the
/// one-frame race), else [`RonSalvagePoll::Settled`] splitting the members into
/// loaded payloads and malformed failures.
#[must_use]
pub fn poll_ron_folder_salvage<'a, T>(
    salvage: &'a RonFolderSalvage<T>,
    asset_server: &AssetServer,
    specs: &'a Assets<RonAsset<T>>,
) -> RonSalvagePoll<'a, T>
where
    T: TypePath + Send + Sync + 'static,
{
    let mut loaded = Vec::new();
    let mut malformed = Vec::new();
    for member in &salvage.members {
        match asset_server.load_state(member.handle.id()) {
            LoadState::Failed(error) => malformed.push(MalformedMember {
                path:  FindingReferrer::new(member.path.as_str().to_owned()),
                error: FindingDetail::new(error.to_string()),
            }),
            LoadState::Loaded => {
                let Some(spec) = specs.get(&member.handle) else {
                    // Loaded-but-not-yet-in-collection — retry next frame
                    // (never publish a partial registry, the C2(c) rule).
                    return RonSalvagePoll::Pending;
                };
                loaded.push(SalvagedMember {
                    path: &member.path,
                    spec,
                });
            }
            LoadState::NotLoaded | LoadState::Loading => return RonSalvagePoll::Pending,
        }
    }
    RonSalvagePoll::Settled { loaded, malformed }
}

/// Rebuild-time view of a salvage for the REDRIVE: like
/// [`poll_ron_folder_salvage`] but a terminally-failed member is silently
/// SKIPPED (it was already reported when the salvage settled) and only a
/// mid-reload member returns [`None`] (keep the existing registry until it
/// settles — the redrive's never-publish-partial rule).
#[must_use]
pub fn salvage_members_for_rebuild<'a, T>(
    salvage: &'a RonFolderSalvage<T>,
    asset_server: &AssetServer,
    specs: &'a Assets<RonAsset<T>>,
) -> Option<Vec<SalvagedMember<'a, T>>>
where
    T: TypePath + Send + Sync + 'static,
{
    match poll_ron_folder_salvage(salvage, asset_server, specs) {
        RonSalvagePoll::Pending => None,
        RonSalvagePoll::Settled { loaded, .. } => Some(loaded),
    }
}

/// `warn!` each salvaged-around malformed member and record it in the
/// [`ContentIntegrityReport`] (when the host carries one) — the C4 loud,
/// non-fatal per-file failure surface, written ONCE for the generic content-family
/// machinery AND the bespoke (non-generic) folder resolvers.
pub fn report_malformed_members(
    report: Option<&mut ContentIntegrityReport>,
    family: &FindingFamily,
    malformed: Vec<MalformedMember>,
) {
    let mut report = report;
    for member in malformed {
        bevy::log::warn!(
            "GDTF Load: `{}` failed to load into {} ({}); its well-formed \
             siblings were salvaged per-file",
            &*member.path,
            &**family,
            &*member.error,
        );
        if let Some(report) = report.as_deref_mut() {
            report.record(ContentFinding::MalformedFile {
                path:   member.path,
                family: family.clone(),
                detail: member.error,
            });
        }
    }
}

/// Enumerate `folder`'s member files matching `.{extension}` (recursively)
/// through the asset server's DEFAULT source reader.
///
/// `block_on` is sound here: the reader's directory walk is local-FS metadata
/// I/O (the same walk `load_folder` runs on an IO task), invoked at most once
/// per family per run, and only on the already-failed error path.
fn enumerate_member_paths(
    asset_server: &AssetServer,
    folder: &str,
    extension: &str,
) -> Result<Vec<String>, AssetReaderError> {
    let source = asset_server
        .get_source(AssetSourceId::Default)
        .map_err(|_| AssetReaderError::NotFound(PathBuf::from(folder)))?;
    let reader = source.reader();
    let suffix = format!(".{extension}");
    let mut paths = Vec::new();
    block_on(collect_members(
        reader,
        Path::new(folder),
        &suffix,
        &mut paths,
    ))?;
    // The reader's directory order is platform-defined; sort so the salvage
    // (and its findings) enumerate deterministically.
    paths.sort();
    Ok(paths)
}

/// Recursively collect every file under `path` whose name ends with `suffix`
/// (the family's `.{extension}`), mirroring the recursive walk
/// `load_folder_internal` runs.
async fn collect_members(
    reader: &dyn ErasedAssetReader,
    path: &Path,
    suffix: &str,
    out: &mut Vec<String>,
) -> Result<(), AssetReaderError> {
    let mut stream = reader.read_directory(path).await?;
    while let Some(child) = stream.next().await {
        if reader.is_directory(&child).await? {
            Box::pin(collect_members(reader, &child, suffix, out)).await?;
        } else {
            // Normalise separators so the suffix filter and the asset-path
            // parse behave identically on every platform.
            let child_path = child.to_string_lossy().replace('\\', "/");
            if child_path.ends_with(suffix) {
                out.push(child_path);
            }
        }
    }
    Ok(())
}

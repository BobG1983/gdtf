//! The ONE kick-off / resolve / redrive system triplet every content family
//! runs, plus the ONE folder-walk builder both resolve and redrive share.
//!
//! Before GTW-570 this triplet (plus two handle newtypes and a poll branch)
//! was hand-stamped once per folder family, each copy re-encoding the same
//! shared behaviors. They now live here exactly once — see
//! [`ContentFamily`](crate::ContentFamily) for the guarantee list.

use core::any::TypeId;

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::*,
};

use crate::{
    asset::RonAsset,
    family::{
        def::{ContentFamily, ContentFileStem},
        handle::ContentFolderHandle,
        report::{ContentIntegrityReport, FindingFamily},
        salvage::{
            MalformedMember, RonFolderSalvage, RonSalvagePoll, begin_ron_folder_salvage,
            poll_ron_folder_salvage, report_malformed_members, salvage_members_for_rebuild,
        },
    },
    hot::short_type_name,
};

/// `Startup`: kick off a content family's folder load, storing its persistent
/// [`ContentFolderHandle`].
///
/// Loads the family's [`FOLDER`](ContentFamily::FOLDER) recursively via
/// [`AssetServer::load_folder`] — every member dispatches to the family's
/// dedicated-extension [`RonAsset`] loader — and inserts the handle the
/// resolve polls and the redrive re-enumerates. Takes
/// `Option<Res<AssetServer>>` so a headless app with no [`AssetServer`] no-ops
/// rather than panicking (GTW-570 C2(a), `bevy-traps.md` #1) — belt-and-braces
/// on top of the registration self-gate.
pub fn kick_off_content_family<F: ContentFamily>(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };
    commands.insert_resource(ContentFolderHandle::<F>::new(
        asset_server.load_folder(F::FOLDER),
    ));
}

/// `Update` (gated until the registry is resolved): resolve the loaded folder
/// into the family's registry resource — inserting it exactly ONCE.
///
/// Registered
/// `run_if(resource_exists::<ContentFolderHandle<F>> AND not(resource_exists::<F::Registry>))`
/// so it inserts once and NEVER re-publishes over live data (the own-absence
/// shadow semantics headless seeds rely on); the live re-derive is
/// [`redrive_content_family`].
///
/// - Gates on the folder's [`RecursiveDependencyLoadState`]`::Loaded`
///   (recursive, so every member IN the folder is loaded). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s naming the folder and
///   begins a PER-FILE SALVAGE (GTW-582 C4): the folder directory is re-walked
///   and every matching member loads INDIVIDUALLY, so one malformed file can no
///   longer empty its family — well-formed siblings still fold into the
///   registry and each malformed member is recorded as a loud
///   [`ContentFinding::MalformedFile`](crate::ContentFinding::MalformedFile). A folder that cannot be enumerated at
///   all (a genuinely missing directory) still fails closed to the EMPTY
///   registry (GTW-570 C2(b), the ADR-0003 error-path safety-net) so a
///   presence-gated `Load` flow never hangs on a bad folder.
/// - On success it walks the folder through the ONE shared builder
///   (`build_family_registry`, private — the redrive shares it); a member
///   still absent from its `Assets` collection returns WITHOUT inserting and
///   retries next frame (C2(c), never-publish-partial), so a partial registry
///   is never published.
/// - Else (still loading) it does nothing and is polled again next frame.
pub fn resolve_content_family<F: ContentFamily>(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    specs: Option<Res<Assets<RonAsset<F::Spec>>>>,
    handle: Option<Res<ContentFolderHandle<F>>>,
    salvage: Option<Res<RonFolderSalvage<F::Spec>>>,
    report: Option<ResMut<ContentIntegrityReport>>,
) {
    let (Some(asset_server), Some(folders), Some(specs), Some(handle)) =
        (asset_server, folders, specs, handle)
    else {
        return;
    };

    // Salvage-poll path: a prior frame's `Failed` began a per-file salvage —
    // settle it (fold the loaded members, report the malformed ones) before
    // ever re-reading the (still-failed) folder state.
    if let Some(salvage) = salvage.as_deref() {
        if let RonSalvagePoll::Settled { loaded, malformed } =
            poll_ron_folder_salvage(salvage, &asset_server, &specs)
        {
            let mut registry = F::Registry::default();
            for member in &loaded {
                F::insert_member(
                    &mut registry,
                    stem_from_path::<F>(member.path.as_str()),
                    member.spec,
                );
            }
            record_malformed_members::<F>(malformed, report);
            commands.insert_resource(registry);
        }
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&**handle);

    // Failure path (GTW-582 C4): a Failed folder walk no longer empties the
    // family — begin the per-file salvage so well-formed siblings still load.
    // Only a folder that cannot be enumerated at all (missing directory) still
    // fails closed to the EMPTY registry, so the flow is never stranded.
    if let RecursiveDependencyLoadState::Failed(error) = folder_state {
        match begin_ron_folder_salvage::<F::Spec>(&asset_server, F::FOLDER, F::EXTENSION) {
            Ok(salvage) if !salvage.is_empty() => {
                warn!(
                    "GDTF Load: the `{}` content folder failed to load ({error}); salvaging \
                     its members per-file into {} (the malformed file alone will fail)",
                    F::FOLDER,
                    short_type_name::<F::Registry>(),
                );
                commands.insert_resource(salvage);
            }
            _ => {
                warn!(
                    "GDTF Load: the `{}` content folder failed to load; inserting an empty {} \
                     (consumers fail closed on a missing key)",
                    F::FOLDER,
                    short_type_name::<F::Registry>(),
                );
                commands.insert_resource(F::Registry::default());
            }
        }
        return;
    }

    // Success path: once every member in the folder is loaded, walk it and
    // publish the registry. The persistent ContentFolderHandle was already
    // inserted by the kick-off and is never removed, so it sits beside the
    // registry for the redrive + the file-watcher.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) = build_family_registry::<F>(&asset_server, &folders, &specs, &handle)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) —
            // retry next frame (the run-condition keeps this system alive while
            // the registry is absent).
            return;
        };
        commands.insert_resource(registry);
    }
}

/// `warn!` each salvaged-around malformed member and record it in the
/// [`ContentIntegrityReport`] (when the host installed one) — the family-typed
/// front of the ONE shared [`report_malformed_members`] surface.
fn record_malformed_members<F: ContentFamily>(
    malformed: Vec<MalformedMember>,
    mut report: Option<ResMut<ContentIntegrityReport>>,
) {
    report_malformed_members(
        report.as_deref_mut(),
        &FindingFamily::new(short_type_name::<F::Registry>().to_owned()),
        malformed,
    );
}

/// `Update` (ungated; self-gates on its [`Option`] borrows): rebuild the
/// family's registry in place on a member
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) — the LIVE
/// hot-reload (GTW-570 C2(e), the GTW-374 Part C convention).
///
/// A folder load fans out into one `RonAsset<F::Spec>` asset PER file, and a
/// hot edit fires a `Modified` for THAT member asset (not the [`LoadedFolder`]
/// handle), so this reacts to ANY `AssetEvent<RonAsset<F::Spec>>::Modified` —
/// the family's dedicated extension makes the spec type family-unique, so every
/// such event IS a member edit — and rebuilds the whole registry from the
/// persistent [`ContentFolderHandle`]'s members via the SAME builder the
/// one-time resolve uses, so a live edit yields the registry a restart would.
/// However many member events arrive in a frame it rebuilds at most ONCE.
/// Overwriting through [`ResMut`] marks the registry CHANGED so change-driven
/// consumers re-derive the same frame, then `info!`s naming the concrete
/// registry type + folder (matching the GTW-564 reload-log wording).
///
/// While any needed resource is still absent (pre-resolve) it DRAINS the
/// reader via `events.clear()` so a stale event never lingers and re-fires
/// once the resources arrive. A rebuild that hits a mid-reload member (absent
/// from its collection) keeps the existing registry until it settles — the
/// next event rebuilds.
pub fn redrive_content_family<F: ContentFamily>(
    mut events: MessageReader<AssetEvent<RonAsset<F::Spec>>>,
    asset_server: Option<Res<AssetServer>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    specs: Option<Res<Assets<RonAsset<F::Spec>>>>,
    handle: Option<Res<ContentFolderHandle<F>>>,
    salvage: Option<Res<RonFolderSalvage<F::Spec>>>,
    registry: Option<ResMut<F::Registry>>,
) {
    let (Some(asset_server), Some(folders), Some(specs), Some(handle), Some(mut registry)) =
        (asset_server, folders, specs, handle, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire
        // once the resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified member — a single rebuild from the latest
    // in-memory specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    // GTW-582 C4: a salvaged family has no `LoadedFolder` asset to re-walk (the
    // folder load failed), so its rebuild enumerates the salvage's per-file
    // member handles instead — a still-failed member is skipped (it was already
    // reported when the salvage settled), so an edit to a healthy sibling still
    // hot-reloads.
    let rebuilt = if let Some(salvage) = salvage.as_deref() {
        let Some(members) = salvage_members_for_rebuild(salvage, &asset_server, &specs) else {
            // A member is mid-reload — leave the existing registry until it
            // settles; the next event rebuilds.
            return;
        };
        let mut rebuilt = F::Registry::default();
        for member in &members {
            F::insert_member(
                &mut rebuilt,
                stem_from_path::<F>(member.path.as_str()),
                member.spec,
            );
        }
        rebuilt
    } else if let Some(rebuilt) =
        build_family_registry::<F>(&asset_server, &folders, &specs, &handle)
    {
        rebuilt
    } else {
        // A member is mid-reload (not yet back in its collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "hot-reload: rebuilt {} from `{}`",
        short_type_name::<F::Registry>(),
        F::FOLDER,
    );
}

/// Build a family's registry from its loaded [`LoadedFolder`], or [`None`] if
/// the folder (or any matching-type member) is not yet in its collection.
///
/// The ONE folder walk both [`resolve_content_family`] (the one-time build)
/// and [`redrive_content_family`] (the live rebuild) share, so both build
/// IDENTICALLY:
///
/// - each member is FIRST filtered by its asset [`TypeId`] — UNCONDITIONALLY
///   (GTW-570 C1): a member whose type is not `RonAsset<F::Spec>` is SKIPPED
///   (a mixed folder's other family handles it), never blindly typed — a
///   wrong-type `typed_debug_checked` would trip its debug assert;
/// - a matching-type member absent from its `Assets` collection forces the
///   [`None`] one-frame retry (never-publish-partial, C2(c));
/// - each present member folds into the registry through the family's
///   [`insert_member`](ContentFamily::insert_member), handed the
///   infix-stripped file stem (or [`None`] for a path-less handle) so both
///   keying shapes resolve without a mode flag.
fn build_family_registry<F: ContentFamily>(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    specs: &Assets<RonAsset<F::Spec>>,
    handle: &ContentFolderHandle<F>,
) -> Option<F::Registry> {
    let folder = folders.get(&**handle)?;

    let mut registry = F::Registry::default();
    for untyped in &folder.handles {
        // The UNCONDITIONAL TypeId filter: skip members of another family's
        // asset type (mixed folders — the terrain/theme tree).
        if untyped.type_id() != TypeId::of::<RonAsset<F::Spec>>() {
            continue;
        }
        let typed = untyped.clone().typed_debug_checked::<RonAsset<F::Spec>>();
        // A matching-type member whose load has not yet landed — bail (do NOT
        // publish a partial registry) so the caller re-polls next frame.
        let spec = specs.get(&typed)?;
        F::insert_member(&mut registry, member_stem::<F>(asset_server, untyped), spec);
    }
    Some(registry)
}

/// A member's [`ContentFileStem`]: its file stem with the family's dedicated
/// infix stripped, or [`None`] when the handle carries no resolvable path.
///
/// The infix is derived from [`EXTENSION`](ContentFamily::EXTENSION)
/// (`weapon.ron` → `.weapon`): `stub_pistol.weapon.ron`'s `file_stem()` is
/// `stub_pistol.weapon`, whose key stem is `stub_pistol`. A stem without the
/// infix is passed through unchanged (defensive — a mis-named file keys by its
/// plain stem, the per-family precedent).
fn member_stem<F: ContentFamily>(
    asset_server: &AssetServer,
    untyped: &bevy::asset::UntypedHandle,
) -> Option<ContentFileStem> {
    let path = asset_server.get_path(untyped.id())?;
    stem_from_path::<F>(path.path().to_string_lossy().as_ref())
}

/// A member's [`ContentFileStem`] computed from its asset-path STRING — the
/// [`member_stem`] core, shared with the per-file salvage paths (whose members
/// carry their path directly rather than an untyped folder handle).
fn stem_from_path<F: ContentFamily>(path: &str) -> Option<ContentFileStem> {
    let stem = std::path::Path::new(path)
        .file_stem()?
        .to_string_lossy()
        .into_owned();
    let infix = F::EXTENSION.strip_suffix(".ron").unwrap_or(F::EXTENSION);
    let suffix = format!(".{infix}");
    let key = stem.strip_suffix(suffix.as_str()).unwrap_or(stem.as_str());
    Some(ContentFileStem::new(key.to_owned()))
}

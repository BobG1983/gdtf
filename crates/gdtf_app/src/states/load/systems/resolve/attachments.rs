//! GTW-549 (PHASE 1): builds the name-keyed [`AttachmentRegistry`] from the loaded
//! `assets/content/attachments/` folder, plus the LIVE hot-reload that rebuilds it on a
//! `*.attachment.ron` edit — the attachment mirror of the sibling
//! [`melee_weapons`](super::melee_weapons) / [`weapons`](super::weapons) resolvers.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::{
    ContentIntegrityReport, FindingFamily, RonAsset, RonFolderSalvage, RonSalvagePoll,
    begin_ron_folder_salvage, poll_ron_folder_salvage, report_malformed_members,
};
use gdtf_battle_sim::weapon::{AttachmentName, AttachmentRegistry, AttachmentSpec};

use crate::states::load::resources::{ActiveAttachmentsFolderHandle, LoadHandles};

/// GTW-549 (PHASE 1): builds the name-keyed [`AttachmentRegistry`] from the loaded
/// `assets/content/attachments/` folder — the GTW-505 melee-weapons resolve shape
/// (that family now rides the GTW-570 generic content-family seam).
///
/// Called only while no [`AttachmentRegistry`] resource exists yet (the caller's own-absence
/// guard), independently of the other resolve branches:
///
/// - Gates on the attachments folder's [`RecursiveDependencyLoadState`]`::Loaded`. On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and begins a PER-FILE SALVAGE
///   (GTW-582 C4, through the ONE shared `gdtf_assets` salvage seam): every matching member
///   loads individually, well-formed siblings still fold into the registry, and each
///   malformed member is recorded as a loud
///   [`MalformedFile`](gdtf_assets::ContentFinding::MalformedFile) finding. A folder that
///   cannot be enumerated at all (missing directory) still fails closed to the EMPTY
///   [`AttachmentRegistry`] so `Load` always exits with one present (the ADR-0003
///   fail-safe; a weapon's authored attachment key then resolves to nothing rather than
///   crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<AttachmentSpec>`, reads its [`AttachmentSpec`] out of the collection, keys it
///   by the asset path's file STEM with the dedicated `.attachment` infix stripped (so
///   `scoped_sight.attachment.ron` keys `scoped_sight`), and inserts every `(AttachmentName,
///   AttachmentSpec)` into the registry. The one-frame loaded-but-not-yet-in-collection race
///   retries next frame (it never publishes a partial registry).
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_attachments(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    attachment_specs: &Assets<RonAsset<AttachmentSpec>>,
    handles: &LoadHandles,
    salvage: Option<&RonFolderSalvage<AttachmentSpec>>,
    report: Option<&mut ContentIntegrityReport>,
) {
    // Salvage-poll path: a prior frame's `Failed` began a per-file salvage — settle it
    // (fold the loaded members, report the malformed ones) before re-reading the folder.
    if let Some(salvage) = salvage {
        if let RonSalvagePoll::Settled { loaded, malformed } =
            poll_ron_folder_salvage(salvage, asset_server, attachment_specs)
        {
            let mut registry = AttachmentRegistry::default();
            for member in &loaded {
                let Some(stem) = std::path::Path::new(member.path.as_str())
                    .file_stem()
                    .map(|stem| attachment_key_from_stem(&stem.to_string_lossy()))
                else {
                    continue;
                };
                registry.insert(AttachmentName::new(stem), (**member.spec).clone());
            }
            report_malformed_members(
                report,
                &FindingFamily::new("AttachmentRegistry".to_owned()),
                malformed,
            );
            commands.insert_resource(registry);
        }
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&*handles.attachments);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        // GTW-582 C4: a Failed folder walk no longer empties the family — salvage the
        // members per-file so one malformed item cannot vanish its siblings. Only a
        // folder that cannot be enumerated at all still fails closed to EMPTY.
        match begin_ron_folder_salvage::<AttachmentSpec>(
            asset_server,
            "content/attachments",
            "attachment.ron",
        ) {
            Ok(salvage) if !salvage.is_empty() => {
                warn!(
                    "GDTF Load: the `content/attachments` folder failed to load; salvaging \
                     its members per-file into the AttachmentRegistry",
                );
                commands.insert_resource(salvage);
            }
            _ => {
                warn!(
                    "GDTF Load: the `content/attachments` folder failed to load; inserting an \
                     empty AttachmentRegistry (a weapon's authored attachment key will resolve \
                     to nothing)",
                );
                commands.insert_resource(AttachmentRegistry::default());
            }
        }
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) = build_attachment_registry(
            asset_server,
            folders,
            attachment_specs,
            &handles.attachments,
        ) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays alive
            // while the AttachmentRegistry is absent).
            return;
        };

        // Persist past OnExit(Load) (NOT removed in cleanup) — PHASE 2 setup reads it.
        commands.insert_resource(registry);
        // GTW-549: insert the PERSISTENT folder handle alongside the registry so the live
        // hot-reload handler can re-enumerate the folder's member handles, and holding it
        // keeps every member attachment asset loaded for the file-watcher.
        commands.insert_resource(ActiveAttachmentsFolderHandle::new(
            (*handles.attachments).clone(),
        ));
    }
}

/// Build the name-keyed [`AttachmentRegistry`] from a loaded `content/attachments/`
/// [`LoadedFolder`], or [`None`] if the folder (or any member spec) is not yet in its
/// collection.
///
/// Shared by [`resolve_attachments`] (the one-time `Load`-state build) and
/// [`redrive_attachments_on_asset_event`] (the live rebuild), so both build the registry
/// IDENTICALLY — the [`build_melee_weapon_registry`](super::melee_weapons) mirror.
fn build_attachment_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    attachment_specs: &Assets<RonAsset<AttachmentSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<AttachmentRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = AttachmentRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<AttachmentSpec> and read its spec.
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<AttachmentSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: bail (do NOT build a partial
        // registry) so the caller re-polls.
        let spec = attachment_specs.get(&handle)?;
        // Key by the file STEM with the dedicated `.attachment` infix stripped:
        // `scoped_sight.attachment.ron`'s `file_stem()` is `scoped_sight.attachment`, whose
        // KEY is `scoped_sight`. A handle with no resolvable path / stem is skipped defensively.
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| attachment_key_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        registry.insert(AttachmentName::new(stem), (**spec).clone());
    }
    Some(registry)
}

/// `Update`: rebuild the [`AttachmentRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/attachments/*.attachment.ron` — the LIVE attachment hot-reload
/// (the GTW-505 melee-weapons redrive shape; that family now rides the GTW-570
/// generic content-family seam).
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes the
/// folder handle / the `Assets` collections / the [`AttachmentRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1). Param-only (`bevy-traps.md` #7).
pub(in crate::states::load) fn redrive_attachments_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<AttachmentSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveAttachmentsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    attachment_specs: Option<Res<Assets<RonAsset<AttachmentSpec>>>>,
    registry: Option<ResMut<AttachmentRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(attachment_specs),
        Some(mut registry),
    ) = (
        asset_server,
        folder_handle,
        folders,
        attachment_specs,
        registry,
    )
    else {
        // Drain so a pre-resolve event does not linger; nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified attachment member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_attachment_registry(&asset_server, &folders, &attachment_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the existing
        // registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "attachment hot-reload: rebuilt AttachmentRegistry from \
         `assets/content/attachments/` ({} attachments)",
        registry.len(),
    );
}

/// The attachment KEY for a loaded attachment file's stem — the stem with the dedicated
/// `.attachment` infix stripped (GTW-549).
///
/// An attachment file is `<key>.attachment.ron`; Bevy's `file_stem()` yields
/// `<key>.attachment`, so the KEY is that stem minus a trailing `.attachment`. A stem without
/// the infix is returned unchanged (defensive).
fn attachment_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".attachment").unwrap_or(stem).to_owned()
}

#[cfg(test)]
mod test;

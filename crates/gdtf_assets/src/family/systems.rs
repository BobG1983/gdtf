//! Startup folder load, resolve (including salvage), and hot-redrive.

use core::any::TypeId;
use std::path::Path;

use bevy::{
    asset::{
        AssetEvent, AssetPath, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState,
    },
    prelude::*,
};
use cobalt_ron_assets::{RonAsset, short_type_name};

use crate::family::{
    def::{ContentFamily, ContentFileStem, ContentMemberKey},
    handle::ContentFolderHandle,
    report::{ContentIntegrityReport, FindingFamily},
    salvage::{
        MalformedMember, RonFolderSalvage, RonSalvagePoll, SalvagedMember,
        begin_ron_folder_salvage, poll_ron_folder_salvage, report_malformed_members,
        salvage_members_for_rebuild,
    },
    source::{ContentSourcePath, ContentSourcePaths, PublishedFamily},
};

/// One member's key and the file it was read from, as a registry build collects them.
type MemberSource = (ContentMemberKey, ContentSourcePath);

/// Begin loading the family's folder on startup.
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

/// Build the registry once the folder (or salvage) is ready.
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

    if let Some(salvage) = salvage.as_deref() {
        if let RonSalvagePoll::Settled { loaded, malformed } =
            poll_ron_folder_salvage(salvage, &asset_server, &specs)
        {
            let (registry, sources) = build_from_salvage::<F>(&loaded);
            record_malformed_members::<F>(malformed, report);
            commands.insert_resource(registry);
            commands.insert_resource(sources);
        }
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&**handle);

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
                commands.insert_resource(ContentSourcePaths::<F>::default());
            }
        }
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some((registry, sources)) =
            build_family_registry::<F>(&asset_server, &folders, &specs, &handle)
        else {
            return;
        };
        commands.insert_resource(registry);
        commands.insert_resource(sources);
    }
}

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

/// Rebuild the registry and its source paths when any member RON is modified.
pub fn redrive_content_family<F: ContentFamily>(
    mut events: MessageReader<AssetEvent<RonAsset<F::Spec>>>,
    asset_server: Option<Res<AssetServer>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    specs: Option<Res<Assets<RonAsset<F::Spec>>>>,
    handle: Option<Res<ContentFolderHandle<F>>>,
    salvage: Option<Res<RonFolderSalvage<F::Spec>>>,
    published: PublishedFamily<F>,
) {
    let PublishedFamily { registry, sources } = published;
    let (
        Some(asset_server),
        Some(folders),
        Some(specs),
        Some(handle),
        Some(mut registry),
        Some(mut sources),
    ) = (asset_server, folders, specs, handle, registry, sources)
    else {
        events.clear();
        return;
    };

    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let rebuilt = if let Some(salvage) = salvage.as_deref() {
        let Some(members) = salvage_members_for_rebuild(salvage, &asset_server, &specs) else {
            return;
        };
        build_from_salvage::<F>(&members)
    } else if let Some(rebuilt) =
        build_family_registry::<F>(&asset_server, &folders, &specs, &handle)
    {
        rebuilt
    } else {
        return;
    };
    let (rebuilt_registry, rebuilt_sources) = rebuilt;
    *registry = rebuilt_registry;
    *sources = rebuilt_sources;
    info!(
        "hot-reload: rebuilt {} from `{}`",
        short_type_name::<F::Registry>(),
        F::FOLDER,
    );
}

// Salvaged members carry their own asset-root-relative path, so the pair builds in one pass.
fn build_from_salvage<F: ContentFamily>(
    members: &[SalvagedMember<'_, F::Spec>],
) -> (F::Registry, ContentSourcePaths<F>) {
    let mut registry = F::Registry::default();
    let mut sources: Vec<MemberSource> = Vec::new();
    for member in members {
        let path = member.path.as_str();
        let key = F::insert_member(&mut registry, stem_from_path::<F>(path), member.spec);
        record_source(&mut sources, key, Some(Path::new(path)));
    }
    (registry, ContentSourcePaths::new(sources))
}

fn build_family_registry<F: ContentFamily>(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    specs: &Assets<RonAsset<F::Spec>>,
    handle: &ContentFolderHandle<F>,
) -> Option<(F::Registry, ContentSourcePaths<F>)> {
    let folder = folders.get(&**handle)?;

    let mut registry = F::Registry::default();
    let mut sources: Vec<MemberSource> = Vec::new();
    for untyped in &folder.handles {
        if untyped.type_id() != TypeId::of::<RonAsset<F::Spec>>() {
            continue;
        }
        let typed = untyped.clone().typed_debug_checked::<RonAsset<F::Spec>>();
        let spec = specs.get(&typed)?;
        let path = asset_server.get_path(untyped.id());
        let stem = path
            .as_ref()
            .and_then(|path| stem_from_path::<F>(path.path().to_string_lossy().as_ref()));
        let key = F::insert_member(&mut registry, stem, spec);
        record_source(&mut sources, key, path.as_ref().map(AssetPath::path));
    }
    Some((registry, ContentSourcePaths::new(sources)))
}

// A member with no key, or none the asset server can name a path for, records nothing.
fn record_source(
    sources: &mut Vec<MemberSource>,
    key: Option<ContentMemberKey>,
    path: Option<&Path>,
) {
    if let (Some(key), Some(path)) = (key, path) {
        sources.push((key, ContentSourcePath::new(path.to_path_buf())));
    }
}

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

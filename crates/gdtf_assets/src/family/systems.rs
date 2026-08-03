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

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) = build_family_registry::<F>(&asset_server, &folders, &specs, &handle)
        else {
            return;
        };
        commands.insert_resource(registry);
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
        return;
    };
    *registry = rebuilt;
    info!(
        "hot-reload: rebuilt {} from `{}`",
        short_type_name::<F::Registry>(),
        F::FOLDER,
    );
}

fn build_family_registry<F: ContentFamily>(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    specs: &Assets<RonAsset<F::Spec>>,
    handle: &ContentFolderHandle<F>,
) -> Option<F::Registry> {
    let folder = folders.get(&**handle)?;

    let mut registry = F::Registry::default();
    for untyped in &folder.handles {
        if untyped.type_id() != TypeId::of::<RonAsset<F::Spec>>() {
            continue;
        }
        let typed = untyped.clone().typed_debug_checked::<RonAsset<F::Spec>>();
        let spec = specs.get(&typed)?;
        F::insert_member(&mut registry, member_stem::<F>(asset_server, untyped), spec);
    }
    Some(registry)
}

fn member_stem<F: ContentFamily>(
    asset_server: &AssetServer,
    untyped: &bevy::asset::UntypedHandle,
) -> Option<ContentFileStem> {
    let path = asset_server.get_path(untyped.id())?;
    stem_from_path::<F>(path.path().to_string_lossy().as_ref())
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

use core::any::TypeId;

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use cobalt_ron_assets::RonAsset;
use gdtf_assets::{
    ContentIntegrityReport, FindingFamily, RonFolderSalvage, RonSalvagePoll,
    begin_ron_folder_salvage, poll_ron_folder_salvage, report_malformed_members,
};
use gdtf_battle_sim::level::{Prefab, PrefabName, PrefabRegistry, PrefabSpec};
use gdtf_content_families::prefabs::{PREFAB_EXTENSION, PREFABS_FOLDER};

use crate::states::load::resources::{ActivePrefabsFolderHandle, LoadHandles};

pub(super) fn resolve_prefabs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpec>>,
    handles: &LoadHandles,
    salvage: Option<&RonFolderSalvage<PrefabSpec>>,
    report: Option<&mut ContentIntegrityReport>,
) {
    if let Some(salvage) = salvage {
        if let RonSalvagePoll::Settled { loaded, malformed } =
            poll_ron_folder_salvage(salvage, asset_server, prefab_specs)
        {
            let mut registry = PrefabRegistry::default();
            for member in &loaded {
                let Some(stem) = std::path::Path::new(member.path.as_str())
                    .file_stem()
                    .map(|stem| prefab_name_from_stem(&stem.to_string_lossy()))
                else {
                    continue;
                };
                registry.insert(Prefab::new(PrefabName::new(stem), (**member.spec).clone()));
            }
            report_malformed_members(
                report,
                &FindingFamily::new("PrefabRegistry".to_owned()),
                malformed,
            );
            commands.insert_resource(registry);
        }
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&*handles.prefabs);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        match begin_ron_folder_salvage::<PrefabSpec>(asset_server, PREFABS_FOLDER, PREFAB_EXTENSION)
        {
            Ok(salvage) if !salvage.is_empty() => {
                warn!(
                    "GDTF Load: the `maps` folder failed to load; salvaging its fragments \
                     per-file into the PrefabRegistry",
                );
                commands.insert_resource(salvage);
            }
            _ => {
                warn!(
                    "GDTF Load: the `maps` folder failed to load; inserting an empty \
                     PrefabRegistry (the assembler will have no fragments to pack)",
                );
                commands.insert_resource(PrefabRegistry::default());
            }
        }
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_prefab_registry(asset_server, folders, prefab_specs, &handles.prefabs)
        else {
            return;
        };

        commands.insert_resource(registry);
        commands.insert_resource(ActivePrefabsFolderHandle::new((*handles.prefabs).clone()));
    }
}

fn build_prefab_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<PrefabRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = PrefabRegistry::default();
    for untyped in &folder.handles {
        if untyped.type_id() != TypeId::of::<RonAsset<PrefabSpec>>() {
            continue;
        }
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<PrefabSpec>>();
        let spec = prefab_specs.get(&handle)?;
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| prefab_name_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        let name = PrefabName::new(stem);
        registry.insert(Prefab::new(name, (**spec).clone()));
    }
    Some(registry)
}

pub(in crate::states::load) fn redrive_prefabs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<PrefabSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActivePrefabsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    prefab_specs: Option<Res<Assets<RonAsset<PrefabSpec>>>>,
    registry: Option<ResMut<PrefabRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(prefab_specs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, prefab_specs, registry)
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

    let Some(rebuilt) =
        build_prefab_registry(&asset_server, &folders, &prefab_specs, &folder_handle)
    else {
        return;
    };
    *registry = rebuilt;
    info!(
        "prefab hot-reload: rebuilt PrefabRegistry from `assets/content/maps/` ({} prefabs)",
        registry.len(),
    );
}

fn prefab_name_from_stem(stem: &str) -> String {
    stem.strip_suffix(".prefab").unwrap_or(stem).to_owned()
}

#[cfg(test)]
mod test;

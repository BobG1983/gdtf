use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info},
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonFolderSalvage};
use gdtf_battle_sim::injuries::{InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting};
use gdtf_content_families::injuries::{
    begin_injuries_salvage, build_injury_data, settle_injuries_salvage,
};

use crate::states::load::resources::{ActiveInjuriesFolderHandle, LoadHandles};

#[expect(
    clippy::too_many_arguments,
    reason = "one folder carries two asset types, so the resolve reads two Assets \
              collections AND (GTW-582) two per-type salvage states + the shared report; \
              the single-type resolvers need only one of each"
)]
pub(super) fn resolve_injuries(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    handles: &LoadHandles,
    salvage: (
        Option<&RonFolderSalvage<InjuryDef>>,
        Option<&RonFolderSalvage<InjuryWeighting>>,
    ),
    report: Option<&mut ContentIntegrityReport>,
) {
    let (def_salvage, weighting_salvage) = salvage;
    if let (Some(def_salvage), Some(weighting_salvage)) = (def_salvage, weighting_salvage) {
        settle_injuries_salvage(
            commands,
            asset_server,
            injury_defs,
            weightings,
            def_salvage,
            weighting_salvage,
            report,
        );
        return;
    }

    let folder_state = asset_server.recursive_dependency_load_state(&*handles.injuries);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        begin_injuries_salvage(commands, asset_server);
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some((registry, tables)) = build_injury_data(
            asset_server,
            folders,
            injury_defs,
            weightings,
            &handles.injuries,
        ) else {
            return;
        };

        commands.insert_resource(registry);
        commands.insert_resource(tables);
        commands.insert_resource(ActiveInjuriesFolderHandle::new((*handles.injuries).clone()));
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one folder carries two asset types, so the redrive reads two AssetEvent \
              readers + two Assets collections + two rebuilt resources; the weapons \
              mirror needs only one of each"
)]
pub(in crate::states::load) fn redrive_injuries_on_asset_event(
    mut def_events: MessageReader<AssetEvent<RonAsset<InjuryDef>>>,
    mut weighting_events: MessageReader<AssetEvent<RonAsset<InjuryWeighting>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveInjuriesFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    injury_defs: Option<Res<Assets<RonAsset<InjuryDef>>>>,
    weightings: Option<Res<Assets<RonAsset<InjuryWeighting>>>>,
    resources: Option<(ResMut<InjuryRegistry>, ResMut<InjuryTables>)>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(injury_defs),
        Some(weightings),
        Some((mut registry, mut tables)),
    ) = (
        asset_server,
        folder_handle,
        folders,
        injury_defs,
        weightings,
        resources,
    )
    else {
        def_events.clear();
        weighting_events.clear();
        return;
    };

    let def_modified = def_events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    let weighting_modified = weighting_events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !def_modified && !weighting_modified {
        return;
    }

    let Some((rebuilt_registry, rebuilt_tables)) = build_injury_data(
        &asset_server,
        &folders,
        &injury_defs,
        &weightings,
        &folder_handle,
    ) else {
        return;
    };
    *registry = rebuilt_registry;
    *tables = rebuilt_tables;
    info!(
        "injury hot-reload: rebuilt InjuryRegistry + InjuryTables from `assets/content/injuries/` \
         ({} injuries, {} weighting buckets)",
        registry.len(),
        tables.len(),
    );
}

#[cfg(test)]
mod test;

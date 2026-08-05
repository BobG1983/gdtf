use bevy::{
    asset::{AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, ResMut, info},
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonFolderSalvage};
use gdtf_battle_sim::injuries::{InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting};
use gdtf_content_families::injuries::{
    begin_injuries_salvage, build_injury_data, settle_injuries_salvage,
};

use super::params::{InjuryAssetEdits, InjuryRebuildSources};
use crate::states::load::resources::{ActiveInjuriesFolderHandle, LoadHandles};

/// The asset collections one injury folder's contents live in.
pub(super) struct InjuryAssets<'a> {
    /// Every loaded folder, including the injury folder itself.
    pub folders:    &'a Assets<LoadedFolder>,
    /// Every loaded injury definition.
    pub defs:       &'a Assets<RonAsset<InjuryDef>>,
    /// Every loaded injury weighting table.
    pub weightings: &'a Assets<RonAsset<InjuryWeighting>>,
}

/// What a partly failed injury folder left behind to salvage.
pub(super) struct InjurySalvage<'a> {
    /// The per-file salvage state for injury definitions.
    pub defs:       Option<&'a RonFolderSalvage<InjuryDef>>,
    /// The per-file salvage state for injury weightings.
    pub weightings: Option<&'a RonFolderSalvage<InjuryWeighting>>,
}

pub(super) fn resolve_injuries(
    commands: &mut Commands,
    asset_server: &AssetServer,
    assets: InjuryAssets<'_>,
    handles: &LoadHandles,
    salvage: InjurySalvage<'_>,
    report: Option<&mut ContentIntegrityReport>,
) {
    if let (Some(def_salvage), Some(weighting_salvage)) = (salvage.defs, salvage.weightings) {
        settle_injuries_salvage(
            commands,
            asset_server,
            assets.defs,
            assets.weightings,
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
            assets.folders,
            assets.defs,
            assets.weightings,
            &handles.injuries,
        ) else {
            return;
        };

        commands.insert_resource(registry);
        commands.insert_resource(tables);
        commands.insert_resource(ActiveInjuriesFolderHandle::new((*handles.injuries).clone()));
    }
}

pub(in crate::states::load) fn redrive_injuries_on_asset_event(
    mut edits: InjuryAssetEdits,
    sources: InjuryRebuildSources,
    resources: Option<(ResMut<InjuryRegistry>, ResMut<InjuryTables>)>,
) {
    let Some((mut registry, mut tables)) = resources else {
        edits.discard();
        return;
    };

    if !*edits.any_modified() {
        return;
    }

    let Some((rebuilt_registry, rebuilt_tables)) = sources.rebuild() else {
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

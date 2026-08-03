//! runs (the GTW-630 "one source, two hosts" shape), so an authored injury /
use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    ecs::system::SystemParam,
    prelude::*,
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonAssetAppExt, RonFolderSalvage};
use gdtf_battle_sim::injuries::{InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting};
use gdtf_content_families::injuries::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, begin_injuries_salvage,
    build_injury_data, settle_injuries_salvage,
};

#[derive(Resource, Deref)]
pub(crate) struct InjuriesFolderHandle(Handle<LoadedFolder>);

/// redrive walk, bundled into one `#[derive(SystemParam)]` (the load gate's
#[derive(SystemParam)]
pub(crate) struct InjuryAssets<'w> {
        server:            Res<'w, AssetServer>,
        folders:           Res<'w, Assets<LoadedFolder>>,
        defs:              Res<'w, Assets<RonAsset<InjuryDef>>>,
        weightings:        Res<'w, Assets<RonAsset<InjuryWeighting>>>,
            def_salvage:       Option<Res<'w, RonFolderSalvage<InjuryDef>>>,
        weighting_salvage: Option<Res<'w, RonFolderSalvage<InjuryWeighting>>>,
}

pub(crate) fn register_injuries(app: &mut App) {
    if app.world().get_resource::<AssetServer>().is_none() {
        app.init_resource::<InjuryRegistry>();
        app.init_resource::<InjuryTables>();
        return;
    }
    app.init_ron_asset_with_extensions::<InjuryDef>(vec![INJURY_DEF_EXTENSION]);
    app.init_ron_asset_with_extensions::<InjuryWeighting>(vec![INJURY_WEIGHTING_EXTENSION]);
    app.init_resource::<ContentIntegrityReport>();
    app.add_systems(Startup, kick_off_injuries).add_systems(
        Update,
        (
            resolve_injuries.run_if(
                resource_exists::<InjuriesFolderHandle>
                    .and_then(not(resource_exists::<InjuryRegistry>)),
            ),
            redrive_injuries,
        ),
    );
}

fn kick_off_injuries(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(InjuriesFolderHandle(server.load_folder(INJURIES_FOLDER)));
}

fn resolve_injuries(
    mut commands: Commands,
    handle: Res<InjuriesFolderHandle>,
    assets: InjuryAssets,
    mut report: Option<ResMut<ContentIntegrityReport>>,
) {
    if let (Some(def_salvage), Some(weighting_salvage)) =
        (&assets.def_salvage, &assets.weighting_salvage)
    {
        settle_injuries_salvage(
            &mut commands,
            &assets.server,
            &assets.defs,
            &assets.weightings,
            def_salvage,
            weighting_salvage,
            report.as_deref_mut(),
        );
        return;
    }

    let folder_state = assets.server.recursive_dependency_load_state(&**handle);
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        begin_injuries_salvage(&mut commands, &assets.server);
        return;
    }
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded)
        && let Some((registry, tables)) = build_injury_data(
            &assets.server,
            &assets.folders,
            &assets.defs,
            &assets.weightings,
            &handle,
        )
    {
        commands.insert_resource(registry);
        commands.insert_resource(tables);
    }
}

fn redrive_injuries(
    mut def_events: MessageReader<AssetEvent<RonAsset<InjuryDef>>>,
    mut weighting_events: MessageReader<AssetEvent<RonAsset<InjuryWeighting>>>,
    handle: Option<Res<InjuriesFolderHandle>>,
    assets: InjuryAssets,
    resources: Option<(ResMut<InjuryRegistry>, ResMut<InjuryTables>)>,
) {
    let (Some(handle), Some((mut registry, mut tables))) = (handle, resources) else {
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
        &assets.server,
        &assets.folders,
        &assets.defs,
        &assets.weightings,
        &handle,
    ) else {
        return;
    };
    *registry = rebuilt_registry;
    *tables = rebuilt_tables;
    info!(
        "editor injury hot-reload: rebuilt InjuryRegistry + InjuryTables \
         ({} injuries, {} weighting buckets)",
        registry.len(),
        tables.len(),
    );
}

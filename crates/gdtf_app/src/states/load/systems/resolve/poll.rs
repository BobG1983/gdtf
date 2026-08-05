use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};
use gdtf_ui::{
    resolve_theme_spec,
    theme::{GdtfTheme, default_theme},
};

use crate::states::load::{
    resources::{FailedAssetPath, LoadFailed, LoadHandles},
    systems::resolve::{
        injuries::{InjuryAssets, InjurySalvage, resolve_injuries},
        params::{LoadAssetCollections, ResolvedResources, SalvageStates},
        prefab::resolve_prefabs,
    },
};

pub(in crate::states::load) fn poll_and_resolve(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: LoadAssetCollections,
    resolved: ResolvedResources,
    handles: Option<Res<LoadHandles>>,
    mut salvage: SalvageStates,
) {
    let (theme_present, injuries_present, prefabs_present) = (
        resolved.theme.is_some(),
        resolved.injuries.is_some(),
        resolved.prefabs.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(folders),
        Some(injury_defs),
        Some(weightings),
        Some(prefab_specs),
        Some(handles),
    ) = (
        asset_server,
        collections.theme,
        collections.folders,
        collections.injury_defs,
        collections.weightings,
        collections.prefab_specs,
        handles,
    )
    else {
        return;
    };

    if !injuries_present {
        resolve_injuries(
            &mut commands,
            &asset_server,
            InjuryAssets {
                folders:    &folders,
                defs:       &injury_defs,
                weightings: &weightings,
            },
            &handles,
            InjurySalvage {
                defs:       salvage.injury_defs.as_deref(),
                weightings: salvage.injury_weightings.as_deref(),
            },
            salvage.report.as_deref_mut(),
        );
    }

    if !prefabs_present {
        resolve_prefabs(
            &mut commands,
            &asset_server,
            &folders,
            &prefab_specs,
            &handles,
            salvage.prefabs.as_deref(),
            salvage.report.as_deref_mut(),
        );
    }

    if theme_present {
        return;
    }

    let theme_state = asset_server.load_state(&*handles.theme);
    let fonts_state = asset_server.recursive_dependency_load_state(&*handles.fonts);

    if theme_state.is_failed() {
        fall_back(
            &mut commands,
            FailedAssetPath::new("core_tuning/ui_theme.tuning.ron"),
            &handles,
        );
        return;
    }
    if matches!(fonts_state, RecursiveDependencyLoadState::Failed(_)) {
        fall_back(&mut commands, FailedAssetPath::new("fonts"), &handles);
        return;
    }

    if matches!(theme_state, LoadState::Loaded)
        && matches!(fonts_state, RecursiveDependencyLoadState::Loaded)
    {
        let Some(spec) = theme_assets.get(&*handles.theme) else {
            return;
        };
        let theme: GdtfTheme = resolve_theme_spec(spec, &asset_server);
        commands.insert_resource(theme);
        commands.insert_resource(handles.theme.clone());
    }
}

fn fall_back(commands: &mut Commands, path: FailedAssetPath, handles: &LoadHandles) {
    warn!(
        "GDTF Load: asset `{}` failed to load; falling back to the const default theme",
        &*path,
    );
    commands.insert_resource(LoadFailed::new(path));
    commands.insert_resource(default_theme());
    commands.insert_resource(handles.theme.clone());
}

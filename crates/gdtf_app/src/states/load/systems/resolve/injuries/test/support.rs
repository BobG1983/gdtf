use bevy::{
    MinimalPlugins,
    asset::{AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting},
    severity::Severity,
};
use gdtf_content_families::injuries::{
    INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, build_injury_data,
};

use super::super::redrive_injuries_on_asset_event;

pub(super) fn injury_def(
    name: &str,
    category: InjuryCategory,
    severity: Severity,
) -> Option<InjuryDef> {
    let ron = format!(
        "(name: \"{name}\", category: {category:?}, severity: {severity:?}, \
         popup_text: \"{name}\", log_text: \"is hurt\", inspect_text: \"{name} -- -1 Aim\", \
         effects: [Modify(stat: Aim, amount: -1)])",
    );
    let parsed = ron::de::from_str::<InjuryDef>(&ron);
    assert!(
        parsed.is_ok(),
        "injury fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

pub(super) fn weighting(
    category: InjuryCategory,
    minor: &[(&str, u32)],
    major: &[(&str, u32)],
    critical: &[(&str, u32)],
) -> Option<InjuryWeighting> {
    let rows = |list: &[(&str, u32)]| {
        list.iter()
            .map(|(k, w)| format!("(injury: \"{k}\", weight: {w})"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let ron = format!(
        "(category: {category:?}, minor: [{}], major: [{}], critical: [{}])",
        rows(minor),
        rows(major),
        rows(critical),
    );
    let parsed = ron::de::from_str::<InjuryWeighting>(&ron);
    assert!(
        parsed.is_ok(),
        "weighting fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// [`weighting`] fixture above is the previous shape (its `#[serde(default)]`
pub(super) fn weighting_in_context(
    category: InjuryCategory,
    context: DamageContext,
    minor: &[(&str, u32)],
    major: &[(&str, u32)],
    critical: &[(&str, u32)],
) -> Option<InjuryWeighting> {
    let rows = |list: &[(&str, u32)]| {
        list.iter()
            .map(|(k, w)| format!("(injury: \"{k}\", weight: {w})"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let ron = format!(
        "(category: {category:?}, context: {context:?}, minor: [{}], major: [{}], \
         critical: [{}])",
        rows(minor),
        rows(major),
        rows(critical),
    );
    let parsed = ron::de::from_str::<InjuryWeighting>(&ron);
    assert!(
        parsed.is_ok(),
        "context weighting fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

pub(super) fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset_with_extensions::<InjuryDef>(vec![INJURY_DEF_EXTENSION])
        .init_ron_asset_with_extensions::<InjuryWeighting>(vec![INJURY_WEIGHTING_EXTENSION])
        .add_systems(Update, redrive_injuries_on_asset_event);
    app
}

pub(super) fn add_def(
    app: &mut App,
    path: &'static str,
    def: InjuryDef,
) -> Handle<RonAsset<InjuryDef>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<InjuryDef>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<InjuryDef>>>()
        .insert(handle.id(), RonAsset::new(def));
    assert!(inserted.is_ok(), "member def insert must succeed");
    handle
}

pub(super) fn add_weighting(
    app: &mut App,
    path: &'static str,
    weighting: InjuryWeighting,
) -> Handle<RonAsset<InjuryWeighting>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<InjuryWeighting>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<InjuryWeighting>>>()
        .insert(handle.id(), RonAsset::new(weighting));
    assert!(inserted.is_ok(), "member weighting insert must succeed");
    handle
}

pub(super) fn add_folder(
    app: &mut App,
    members: &[bevy::asset::UntypedHandle],
) -> Handle<LoadedFolder> {
    let folder = LoadedFolder {
        handles: members.to_vec(),
    };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

pub(super) fn build(
    app: &App,
    folder: &Handle<LoadedFolder>,
) -> Option<(InjuryRegistry, InjuryTables)> {
    let world = app.world();
    build_injury_data(
        world.resource::<AssetServer>(),
        world.resource::<Assets<LoadedFolder>>(),
        world.resource::<Assets<RonAsset<InjuryDef>>>(),
        world.resource::<Assets<RonAsset<InjuryWeighting>>>(),
        folder,
    )
}

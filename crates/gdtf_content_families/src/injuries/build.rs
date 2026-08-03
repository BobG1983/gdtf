use bevy::{
    asset::{AssetServer, Assets, LoadedFolder},
    prelude::warn,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    injuries::{
        DamageContext, InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeighting,
        WeightedInjuryEntry, WeightedInjuryTable,
    },
    severity::Severity,
};

use super::keying::{injury_key_from_stem, warn_on_subfolder_mismatch};

#[must_use]
pub fn build_injury_data(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<(InjuryRegistry, InjuryTables)> {
    let folder = folders.get(folder_handle)?;

    let mut registry = InjuryRegistry::default();
    let mut authored_weightings: Vec<InjuryWeighting> = Vec::new();

    for untyped in &folder.handles {
        let Some(asset_path) = asset_server.get_path(untyped.id()) else {
            continue;
        };
        let path = asset_path.path();
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".injury.ron") {
            let handle = untyped.clone().typed_debug_checked::<RonAsset<InjuryDef>>();
            let def = injury_defs.get(&handle)?;
            let Some(stem) = path
                .file_stem()
                .map(|stem| injury_key_from_stem(&stem.to_string_lossy()))
            else {
                continue;
            };
            warn_on_subfolder_mismatch(&path_str, &stem, def);
            registry.insert(InjuryName::new(stem), (**def).clone());
        } else if path_str.ends_with(".weighting.ron") {
            let handle = untyped
                .clone()
                .typed_debug_checked::<RonAsset<InjuryWeighting>>();
            let weighting = weightings.get(&handle)?;
            authored_weightings.push((**weighting).clone());
        }
    }

    let tables = build_tables(&registry, &authored_weightings);
    audit_unweighted_injuries(&registry, &tables);
    Some((registry, tables))
}

pub(super) fn build_tables(
    registry: &InjuryRegistry,
    weightings: &[InjuryWeighting],
) -> InjuryTables {
    let mut tables = InjuryTables::default();
    for weighting in weightings {
        for (severity, rows) in [
            (Severity::Minor, &weighting.minor),
            (Severity::Major, &weighting.major),
            (Severity::Critical, &weighting.critical),
        ] {
            let mut resolved: Vec<WeightedInjuryEntry> = rows
                .iter()
                .filter(|row| {
                    let known = registry.contains(&row.injury);
                    if !known {
                        warn!(
                            "GDTF Load: weighting for {:?} names unknown injury key {:?}; \
                             skipping the row (it resolves to no injury)",
                            severity, &*row.injury,
                        );
                    }
                    known
                })
                .cloned()
                .collect();
            resolved.sort_by(|a, b| {
                a.injury
                    .cmp(&b.injury)
                    .then_with(|| a.weight.cmp(&b.weight))
            });
            if !resolved.is_empty() {
                tables.insert(
                    weighting.category,
                    weighting.context,
                    severity,
                    WeightedInjuryTable::new(resolved),
                );
            }
        }
    }
    tables
}

pub(super) fn audit_unweighted_injuries(registry: &InjuryRegistry, tables: &InjuryTables) {
    for (key, def) in registry.iter() {
        let weighted = DamageContext::ALL.into_iter().any(|context| {
            tables
                .table_for_category(def.category, context, def.severity)
                .is_some_and(|table| table.iter().any(|row| row.injury == *key))
        });
        if !weighted {
            warn!(
                "GDTF Load: injury {:?} ({:?}/{:?}) is in no weighting table (any context); it \
                 can never be rolled",
                &**key, def.category, def.severity,
            );
        }
    }
}

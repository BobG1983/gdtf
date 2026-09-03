//! The injury weighting delete: no record names a table, so its check always clears.

use std::path::Path;

use bevy::{asset::Assets, ecs::change_detection::Mut, prelude::World};
use gdtf_assets::{ContentSourcePath, FindingFamily, RonAsset};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables, InjuryWeighting, WeightedInjuryTable},
    severity::Severity,
};
use gdtf_content_families::injuries::weighting_key_parts;

use crate::{
    delete::registry::{DeleteEntry, DeleteScreen},
    injury_form::weighting_save_path_in,
    mode::{EditorMode, InjurySubTab},
};

/// The finding family label every injury weighting finding carries.
pub(crate) const WEIGHTING_FAMILY: &str = "InjuryTables";

/// The three severities a weighting table authors rows for.
const WEIGHTED: [Severity; 3] = [Severity::Minor, Severity::Major, Severity::Critical];

// What a taken weighting table holds while the delete waits on the re-check.
struct TakenWeighting {
    authored: Vec<InjuryWeighting>,
    buckets:  Vec<(Severity, WeightedInjuryTable)>,
}

/// The delete for one weighting table, offered on the Injury tab's Tables sub-tab.
pub(crate) fn weighting_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(WEIGHTING_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Injury, Some(InjurySubTab::Tables)),
        Box::new(|world, key| {
            let (category, context) = weighting_key_parts(key)?;
            let taken = take_weighting(world, category, context);
            Some(Box::new(taken))
        }),
        Box::new(|world, key, record| {
            let Ok(taken) = record.downcast::<TakenWeighting>() else {
                return;
            };
            let Some((category, context)) = weighting_key_parts(key) else {
                return;
            };
            restore_weighting(world, category, context, *taken);
        }),
        Box::new(|_world, key| {
            let (category, context) = weighting_key_parts(key)?;
            Some(ContentSourcePath::new(weighting_save_path_in(
                Path::new(""),
                category,
                context,
            )))
        }),
    )
}

// Drop both the resolved buckets and the authored asset the re-check would re-read.
fn take_weighting(
    world: &mut World,
    category: InjuryCategory,
    context: DamageContext,
) -> TakenWeighting {
    let buckets = world
        .get_resource_mut::<InjuryTables>()
        .map(|mut tables| take_buckets(&mut tables, category, context))
        .unwrap_or_default();
    let authored = world
        .get_resource_mut::<Assets<RonAsset<InjuryWeighting>>>()
        .map(|mut assets| take_authored(&mut assets, category, context))
        .unwrap_or_default();
    TakenWeighting { authored, buckets }
}

// A bucket per severity the table held, taken out through the watched resource.
fn take_buckets(
    tables: &mut Mut<InjuryTables>,
    category: InjuryCategory,
    context: DamageContext,
) -> Vec<(Severity, WeightedInjuryTable)> {
    WEIGHTED
        .into_iter()
        .filter_map(|severity| {
            tables
                .remove(category, context, severity)
                .map(|table| (severity, table))
        })
        .collect()
}

// The weighting asset raises no event when its file goes, so the delete drops it here.
fn take_authored(
    assets: &mut Mut<Assets<RonAsset<InjuryWeighting>>>,
    category: InjuryCategory,
    context: DamageContext,
) -> Vec<InjuryWeighting> {
    let ids: Vec<_> = assets
        .iter()
        .filter(|(_id, weighting)| weighting.category == category && weighting.context == context)
        .map(|(id, _weighting)| id)
        .collect();
    let mut taken = Vec::new();
    for id in ids {
        if let Some(asset) = assets.remove(id) {
            taken.push((*asset).clone());
        }
    }
    taken
}

// Put both halves back, so a refused delete leaves the world as it found it.
fn restore_weighting(
    world: &mut World,
    category: InjuryCategory,
    context: DamageContext,
    taken: TakenWeighting,
) {
    if let Some(mut tables) = world.get_resource_mut::<InjuryTables>() {
        for (severity, table) in taken.buckets {
            tables.insert(category, context, severity, table);
        }
    }
    if let Some(mut assets) = world.get_resource_mut::<Assets<RonAsset<InjuryWeighting>>>() {
        for weighting in taken.authored {
            assets.add(RonAsset::new(weighting));
        }
    }
}

//! The injury delete: every weighting row naming it is written back without that row.

use bevy::{
    asset::{AssetServer, Assets},
    prelude::World,
};
use gdtf_assets::{ContentMemberKey, ContentSourcePath, FindingFamily, RonAsset};
use gdtf_battle_sim::injuries::{InjuryDef, InjuryName, InjuryRegistry, InjuryWeighting};

use crate::{
    delete::{
        registry::{DeleteEntry, DeleteScreen, take_matching_assets},
        resolution::{DroppedReferences, delete_assets_root},
    },
    injury_form::{injury_file_name, write_weighting_in},
    mode::{EditorMode, InjurySubTab},
};

/// The finding family label every injury reference finding carries.
pub(crate) const INJURY_FAMILY: &str = "InjuryRegistry";

/// The delete for one injury def, offered on the Injury tab's Injury sub-tab.
pub(crate) fn injury_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(INJURY_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Injury, Some(InjurySubTab::Def)),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<InjuryRegistry>()?;
            let def = registry.remove(&injury_name(key))?;
            Some(Box::new(def))
        }),
        Box::new(|world, key, record| {
            let Ok(def) = record.downcast::<InjuryDef>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<InjuryRegistry>() {
                registry.insert(injury_name(key), *def);
            }
        }),
        Box::new(injury_file_path),
    )
    .with_drop(Box::new(|world, key| {
        drop_injury_refs(world, &injury_name(key))
    }))
}

// The registry key the member key names.
fn injury_name(key: &ContentMemberKey) -> InjuryName {
    InjuryName::new((**key).clone())
}

// The injury def loads by folder, so its file is the asset path with a matching name.
fn injury_file_path(world: &World, key: &ContentMemberKey) -> Option<ContentSourcePath> {
    let assets = world.get_resource::<Assets<RonAsset<InjuryDef>>>()?;
    let server = world.get_resource::<AssetServer>()?;
    let wanted = injury_file_name(&injury_name(key));
    assets.iter().find_map(|(id, _def)| {
        let path = server.get_path(id)?;
        let path = path.path();
        (path.file_name()?.to_string_lossy() == wanted)
            .then(|| ContentSourcePath::new(path.to_path_buf()))
    })
}

// Rewrite every weighting table holding a row for the injury, file then asset store.
fn drop_injury_refs(world: &mut World, injury: &InjuryName) -> DroppedReferences {
    let Some(root) = delete_assets_root(world) else {
        return DroppedReferences::Failed;
    };
    let Some(mut assets) = world.get_resource_mut::<Assets<RonAsset<InjuryWeighting>>>() else {
        return DroppedReferences::Nothing;
    };
    let taken: Vec<InjuryWeighting> =
        take_matching_assets(&mut assets, |weighting| names_injury(weighting, injury))
            .iter()
            .map(|asset| (**asset).clone())
            .collect();
    if taken.is_empty() {
        return DroppedReferences::Nothing;
    }
    let rewritten: Vec<InjuryWeighting> = taken
        .iter()
        .map(|weighting| without_injury(weighting, injury))
        .collect();
    let written = rewritten
        .iter()
        .all(|weighting| write_weighting_in(&root, weighting).is_ok());
    let resident = if written { &rewritten } else { &taken };
    if let Some(mut assets) = world.get_resource_mut::<Assets<RonAsset<InjuryWeighting>>>() {
        for weighting in resident {
            assets.add(RonAsset::new(weighting.clone()));
        }
    }
    if written {
        DroppedReferences::Rewritten
    } else {
        DroppedReferences::Failed
    }
}

// Whether any of the three buckets weights the injury.
fn names_injury(weighting: &RonAsset<InjuryWeighting>, injury: &InjuryName) -> bool {
    [&weighting.minor, &weighting.major, &weighting.critical]
        .into_iter()
        .any(|rows| rows.iter().any(|row| row.injury == *injury))
}

// The same table with every row naming the injury gone from all three buckets.
fn without_injury(weighting: &InjuryWeighting, injury: &InjuryName) -> InjuryWeighting {
    let mut next = weighting.clone();
    for rows in [&mut next.minor, &mut next.major, &mut next.critical] {
        rows.retain(|row| row.injury != *injury);
    }
    next
}

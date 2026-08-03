use bevy::prelude::warn;
use gdtf_battle_sim::{armor::InjuryCategory, injuries::InjuryDef};

use super::layout::category_dir;

pub(super) fn injury_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".injury").unwrap_or(stem).to_owned()
}

pub(super) fn warn_on_subfolder_mismatch(path_str: &str, key: &str, def: &InjuryDef) {
    let Some(subfolder) = subfolder_injury_category(path_str) else {
        return;
    };
    if subfolder != def.category {
        warn!(
            "GDTF Load: injury {key:?} declares category {:?} but lives in the {:?} \
             subfolder; loading it under its authoritative field ({:?})",
            def.category, subfolder, def.category,
        );
    }
}

fn subfolder_injury_category(path_str: &str) -> Option<InjuryCategory> {
    let normalised = path_str.replace('\\', "/");
    InjuryCategory::ALL.into_iter().find(|category| {
        let needle = format!("/{}/", category_dir(*category));
        normalised.contains(&needle)
    })
}

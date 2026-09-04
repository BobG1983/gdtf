use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryEffect, InjuryName},
};
use gdtf_content_families::injuries::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, WEIGHTING_SUBFOLDER,
};

use crate::injury_form::{
    draft::DEFAULT_EFFECT,
    save::{injury_file_name, injury_save_path_in, weighting_file_name, weighting_save_path_in},
};

#[test]
fn save_file_names_and_paths_derive_from_the_one_owner_spellings() {
    let key = InjuryName::new("twisted_ankle".to_owned());
    assert_eq!(
        injury_file_name(&key),
        format!("twisted_ankle.{INJURY_DEF_EXTENSION}"),
    );

    let hostile = InjuryName::new("../../Evil Wound!".to_owned());
    assert_eq!(injury_file_name(&hostile), "evil_wound.injury.ron");
    let path = injury_save_path_in(
        std::path::Path::new("/tmp/root"),
        InjuryCategory::Leg,
        &hostile,
    );
    let expected_tail = std::path::Path::new(INJURIES_FOLDER)
        .join("leg")
        .join("evil_wound.injury.ron");
    assert!(
        path.ends_with(&expected_tail),
        "the sanitized def lands under its category subfolder: {path:?}",
    );

    let unnameable = InjuryName::new("!!!///".to_owned());
    assert_eq!(injury_file_name(&unnameable), "unnamed_injury.injury.ron");

    assert_eq!(
        weighting_file_name(InjuryCategory::Torso, DamageContext::Ranged),
        format!("torso.{INJURY_WEIGHTING_EXTENSION}"),
    );
    assert_eq!(
        weighting_file_name(InjuryCategory::Torso, DamageContext::Melee),
        format!("torso.melee.{INJURY_WEIGHTING_EXTENSION}"),
        "a per-source pick must save to its OWN file, never overwrite the ranged one",
    );
    let weighting_path = weighting_save_path_in(
        std::path::Path::new("/tmp/root"),
        InjuryCategory::Torso,
        DamageContext::Ranged,
    );
    let expected_tail = std::path::Path::new(INJURIES_FOLDER)
        .join(WEIGHTING_SUBFOLDER)
        .join("torso.weighting.ron");
    assert!(
        weighting_path.ends_with(&expected_tail),
        "the weighting table lands under the weighting subfolder: {weighting_path:?}",
    );

    assert!(matches!(DEFAULT_EFFECT, InjuryEffect::Modify { .. }));
}

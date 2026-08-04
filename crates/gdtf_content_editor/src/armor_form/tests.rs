use gdtf_assets::{ContentFamily, serialize_ron_pretty};
use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection, ArmorSpec,
    ArmorType, BodyPart,
};
use gdtf_content_families::ArmorFamily;

use super::{
    draft::ArmorDraft,
    save::{armor_file_name, armor_save_path_in, draft_to_spec},
};

fn fixture_spec() -> ArmorSpec {
    let piece = |floor: i32, protection: i32, integrity: i32, hardness: i32, ty: ArmorType| {
        ArmorPiece::new(
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(hardness),
            ty,
        )
    };
    ArmorSpec::new([
        piece(2, 3, 50, 1, ArmorType::Flak),
        piece(1, 4, 60, 2, ArmorType::Plated),
        piece(0, 2, 45, 1, ArmorType::Ceramic),
        piece(1, 2, 40, 0, ArmorType::Void),
        piece(0, 1, 35, 1, ArmorType::Hazard),
        piece(2, 1, 30, 2, ArmorType::Reinforced),
    ])
}

#[test]
fn default_is_pristine_and_load_armor_fills_the_form() {
    let mut draft = ArmorDraft::default();
    assert!(
        draft.autoload_pending(),
        "the fresh seed must autoload once"
    );
    assert_eq!(draft.name(), "");

    let spec = fixture_spec();
    draft.load_armor(&ArmorName::new("edited_plate".to_owned()), &spec);
    assert!(
        !draft.autoload_pending(),
        "a loaded armor ends the autoload"
    );
    assert_eq!(draft.name(), "edited_plate");
    assert_eq!(draft.spec(), &spec);

    let minted = ArmorDraft::new_armor();
    assert!(
        !minted.autoload_pending(),
        "a deliberate new suit never re-seeds"
    );
    assert_eq!(minted.name(), "");

    let mut pristine = ArmorDraft::default();
    pristine.mark_autoloaded();
    assert!(
        !pristine.autoload_pending(),
        "the empty-registry branch ends the seed"
    );
}

#[test]
fn piece_mut_edits_exactly_the_addressed_part() {
    let mut draft = ArmorDraft::new_armor();
    draft.piece_mut(BodyPart::Torso).protection = ArmorProtection::new(7);
    draft.piece_mut(BodyPart::LeftLeg).armor_type = ArmorType::Hazard;

    let (_, spec) = draft_to_spec(&draft);
    assert_eq!(
        *spec.torso.protection, 7,
        "the torso edit lands on the torso"
    );
    assert_eq!(spec.left_leg.armor_type, ArmorType::Hazard);
    assert_eq!(*spec.head.protection, 0);
    assert_eq!(spec.right_leg.armor_type, ArmorType::Plated);
}

#[test]
fn edited_armor_round_trips_through_the_loader_schema() {
    let mut edited = ArmorDraft::new_armor();
    edited.load_armor(&ArmorName::new("edited_plate".to_owned()), &fixture_spec());

    let (name, spec) = draft_to_spec(&edited);
    let serialized = serialize_ron_pretty(&spec);
    assert!(
        serialized.is_ok(),
        "serializing the edited armor must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded_spec = ron::de::from_str::<ArmorSpec>(&serialized);
    assert!(
        reloaded_spec.is_ok(),
        "the serialized armor must round-trip through the ArmorSpec deserializer: {:?}",
        reloaded_spec.as_ref().err(),
    );
    let Ok(reloaded_spec) = reloaded_spec else {
        return;
    };

    let mut reloaded = ArmorDraft::new_armor();
    reloaded.load_armor(&name, &reloaded_spec);
    assert_eq!(
        reloaded, edited,
        "the reloaded armor must equal the edited draft (name + all six per-part pieces)",
    );
}

#[test]
fn save_file_name_and_path_derive_from_the_one_owner_spellings() {
    let name = ArmorName::new("carapace_plate".to_owned());
    assert_eq!(
        armor_file_name(&name),
        format!("carapace_plate.{}", ArmorFamily::EXTENSION),
    );

    let hostile = ArmorName::new("../../Evil Plate!".to_owned());
    assert_eq!(armor_file_name(&hostile), "evil_plate.armor.ron");
    let path = armor_save_path_in(std::path::Path::new("/tmp/root"), &hostile);
    let expected_tail = std::path::Path::new(ArmorFamily::FOLDER).join("evil_plate.armor.ron");
    assert!(
        path.ends_with(&expected_tail),
        "the sanitized file lands under the armor family folder: {path:?}",
    );

    let unnameable = ArmorName::new("!!!///".to_owned());
    assert_eq!(armor_file_name(&unnameable), "unnamed_armor.armor.ron");
}

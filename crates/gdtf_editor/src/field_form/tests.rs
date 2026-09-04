use std::num::NonZeroU8;

use gdtf_assets::ContentFamily;
use gdtf_battle_sim::{
    armor::ArmorType,
    effects::fields::{
        FieldDamage, FieldDef, FieldDuration, FieldKey, FieldTurns, ImmuneArmorTypes,
    },
    weapon::DamageType,
};
use gdtf_content_families::FieldsFamily;

use super::{
    draft::FieldDraft,
    save::{draft_to_field, field_file_name, field_save_path_in, write_field_in},
};

fn turns(count: u8) -> FieldDuration {
    let Some(count) = NonZeroU8::new(count) else {
        unreachable!("every case here names a positive turn count");
    };
    FieldDuration::Turns(FieldTurns::new(count))
}

fn fixture_def() -> FieldDef {
    FieldDef::new(
        FieldDamage::new(4),
        DamageType::Chem,
        ImmuneArmorTypes::new([ArmorType::Flak, ArmorType::Hazard]),
        turns(3),
    )
}

#[test]
fn default_is_pristine_and_load_field_fills_every_getter() {
    let mut draft = FieldDraft::default();
    assert!(
        draft.autoload_pending(),
        "the fresh seed must autoload once"
    );

    draft.load_field(&FieldKey::new("toxic_sump".to_owned()), &fixture_def());
    assert!(
        !draft.autoload_pending(),
        "a loaded field ends the autoload"
    );
    assert_eq!(draft.key(), "toxic_sump");
    assert_eq!(draft.damage(), FieldDamage::new(4));
    assert_eq!(draft.damage_type(), DamageType::Chem);
    assert_eq!(draft.duration(), turns(3));
    assert_eq!(
        draft.immune_armor_types(),
        vec![ArmorType::Flak, ArmorType::Hazard],
        "the immune list reads back in ArmorType::ALL order, not hash order",
    );

    let minted = FieldDraft::new_field();
    assert!(
        !minted.autoload_pending(),
        "a deliberate new field never re-seeds, so the autoload cannot overwrite it",
    );
    assert_eq!(minted.key(), "");
    assert!(minted.immune_armor_types().is_empty());
    assert_eq!(minted.duration(), FieldDuration::Permanent);

    let mut pristine = FieldDraft::default();
    pristine.mark_autoloaded();
    assert!(
        !pristine.autoload_pending(),
        "the empty-registry branch ends the seed",
    );
}

#[test]
fn every_setter_writes_only_the_value_it_names() {
    let mut draft = FieldDraft::new_field();
    draft.set_key("shock_grid".to_owned());
    draft.set_damage(FieldDamage::new(7));
    draft.set_damage_type(DamageType::Shock);
    draft.set_duration(turns(2));
    draft.toggle_immune_armor_type(ArmorType::Plated);

    let (key, def) = draft_to_field(&draft);
    assert_eq!(*key, "shock_grid".to_owned());
    assert_eq!(def.damage, FieldDamage::new(7));
    assert_eq!(def.damage_type, DamageType::Shock);
    assert_eq!(def.duration, turns(2));
    assert!(def.immune_armor_types.contains(&ArmorType::Plated));
    assert!(!def.immune_armor_types.contains(&ArmorType::Flak));
}

#[test]
fn toggling_an_immune_type_twice_leaves_the_list_empty() {
    let mut draft = FieldDraft::new_field();
    draft.toggle_immune_armor_type(ArmorType::Void);
    assert!(draft.is_immune(ArmorType::Void), "the first toggle adds");
    draft.toggle_immune_armor_type(ArmorType::Void);
    assert!(
        !draft.is_immune(ArmorType::Void),
        "the second toggle removes, so an add-only toggle fails here",
    );
    assert!(draft.immune_armor_types().is_empty());
}

#[test]
fn save_file_name_and_path_derive_from_the_family_the_loader_reads() {
    let key = FieldKey::new("toxic_waste_pool".to_owned());
    assert_eq!(
        field_file_name(&key),
        format!("toxic_waste_pool.{}", FieldsFamily::EXTENSION),
    );

    let hostile = FieldKey::new("../../Evil Pool!".to_owned());
    assert_eq!(field_file_name(&hostile), "evil_pool.field.ron");
    let path = field_save_path_in(std::path::Path::new("/tmp/root"), &hostile);
    let expected_tail = std::path::Path::new(FieldsFamily::FOLDER).join("evil_pool.field.ron");
    assert!(
        path.ends_with(&expected_tail),
        "the sanitized file lands under the fields family folder: {path:?}",
    );

    let unnameable = FieldKey::new("!!!///".to_owned());
    assert_eq!(field_file_name(&unnameable), "unnamed_field.field.ron");
}

#[test]
fn a_written_field_reads_back_as_the_def_and_writes_the_same_bytes_twice() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut draft = FieldDraft::new_field();
    draft.load_field(&FieldKey::new("temp_pool".to_owned()), &fixture_def());
    let (key, def) = draft_to_field(&draft);

    let written = write_field_in(dir.path(), &key, &def);
    assert!(
        written.is_ok(),
        "the real field write must succeed: {:?}",
        written.as_ref().err(),
    );
    let Ok(path) = written else { return };

    let first = std::fs::read_to_string(&path);
    assert!(first.is_ok(), "the writer left a readable file at {path:?}");
    let Ok(first) = first else { return };

    let reloaded = ron::de::from_str::<FieldDef>(&first);
    assert_eq!(
        reloaded.ok(),
        Some(def.clone()),
        "the written file parses back as the def the draft would save",
    );

    let rewritten = write_field_in(dir.path(), &key, &def);
    assert!(rewritten.is_ok(), "the second write must succeed too");
    let second = std::fs::read_to_string(&path);
    assert_eq!(
        second.ok(),
        Some(first),
        "two writes of the same draft produce the same bytes, so re-saving a field never churns \
         the file",
    );
}

#[test]
fn the_written_immune_list_runs_in_armor_type_all_order() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut draft = FieldDraft::new_field();
    draft.set_key("every_type".to_owned());
    for armor_type in ArmorType::ALL {
        draft.toggle_immune_armor_type(armor_type);
    }
    let (key, def) = draft_to_field(&draft);

    let written = write_field_in(dir.path(), &key, &def);
    assert!(
        written.is_ok(),
        "the real field write must succeed: {:?}",
        written.as_ref().err(),
    );
    let Ok(path) = written else { return };
    let text = std::fs::read_to_string(&path);
    assert!(text.is_ok(), "the writer left a readable file at {path:?}");
    let Ok(text) = text else { return };

    let mut offsets = Vec::new();
    for armor_type in ArmorType::ALL {
        let name = format!("{armor_type:?}");
        let Some(at) = text.find(&name) else {
            unreachable!("every immune type the draft holds is written: {name} missing from {text}")
        };
        offsets.push(at);
    }
    let mut sorted = offsets.clone();
    sorted.sort_unstable();
    assert_eq!(
        offsets, sorted,
        "the members are written in ArmorType::ALL order, so a writer that walked the set's own \
         hash order fails here: {text}",
    );
}

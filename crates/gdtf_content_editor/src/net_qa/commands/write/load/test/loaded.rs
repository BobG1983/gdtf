use std::path::PathBuf;

use super::fixtures::{
    SEEDED_KEY, TERRAIN_SOURCE_FILE, armor, asked_key, attachments, gangs, injuries, melee_weapons,
    seeded_key, sprites, terrain, terrain_def, terrain_sources, weapons,
};
use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    gang_form::GangDraft,
    injury_form::InjuryDraft,
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::commands::write::load::families::{
        KeyLookup, load_armor, load_attachment, load_gang, load_injury, load_melee_weapon,
        load_sprite, load_terrain, load_weapon,
    },
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    weapon_form::WeaponDraft,
};

// A default draft is still waiting to be seeded; only a loaded draft is settled.
const STILL_PENDING: &str = "the load helper left the draft's autoload flag Pending, so the form's own sync would seed \
     the first sorted registry entry over the loaded entry on the next egui frame — which is \
     what filling the draft any way other than through its own `load_*` method does";

#[test]
fn a_loaded_gang_is_not_reseeded_by_the_form() {
    let mut draft = GangDraft::default();
    let found = load_gang(&mut draft, &gangs(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_armor_is_not_reseeded_by_the_form() {
    let mut draft = ArmorDraft::default();
    let found = load_armor(&mut draft, &armor(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_injury_is_not_reseeded_by_the_form() {
    let mut draft = InjuryDraft::default();
    let found = load_injury(&mut draft, &injuries(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_sprite_is_not_reseeded_by_the_form() {
    let mut draft = SpriteDraft::default();
    let found = load_sprite(&mut draft, &sprites(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_attachment_is_not_reseeded_by_the_form() {
    let mut draft = AttachmentDraft::default();
    let found = load_attachment(&mut draft, &attachments(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_weapon_is_not_reseeded_by_the_form() {
    let mut draft = WeaponDraft::default();
    let found = load_weapon(&mut draft, &weapons(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_melee_weapon_is_not_reseeded_by_the_form() {
    let mut draft = MeleeWeaponDraft::default();
    let found = load_melee_weapon(&mut draft, &melee_weapons(), &seeded_key());
    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {SEEDED_KEY}");
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_loaded_terrain_carries_the_file_it_was_read_from() {
    let def = terrain_def();
    let registry = terrain(&def);
    let sources = terrain_sources(&def);
    let mut draft = TerrainDraft::default();

    let key = asked_key(&(*def.key).to_string());
    let found = load_terrain(&mut draft, &registry, Some(&sources), &key);

    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {key:?}");
    assert_eq!(
        draft.source().map(|path| (**path).clone()),
        Some(PathBuf::from(TERRAIN_SOURCE_FILE)),
        "the load records the file the def was read from; a load that fills the draft and drops \
         the path leaves the next save rebuilding one from the display name",
    );
}

#[test]
fn a_terrain_loaded_with_no_path_table_carries_no_file() {
    let def = terrain_def();
    let registry = terrain(&def);
    let mut draft = TerrainDraft::default();

    let key = asked_key(&(*def.key).to_string());
    let found = load_terrain(&mut draft, &registry, None, &key);

    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {key:?}");
    assert_eq!(
        draft.source(),
        None,
        "with no path table in the world the load still fills the draft, and the save falls \
         back to the display name rather than writing an invented path",
    );
}

#[test]
fn a_key_the_registry_does_not_hold_leaves_the_draft_untouched() {
    let before = ArmorDraft::default();
    let mut draft = before.clone();
    let found = load_armor(&mut draft, &armor(), &asked_key("no_such_armor"));
    let KeyLookup::NoSuchKey(known) = found else {
        unreachable!("`no_such_armor` is not in the fixture registry, got {found:?}");
    };
    assert!(
        known.iter().any(|key| **key == *SEEDED_KEY),
        "a miss lists every key the registry does hold, so one round trip fixes the call: \
         {known:?}",
    );
    assert_eq!(
        draft, before,
        "a miss writes nothing into the draft, so the form still holds what it held before",
    );
}

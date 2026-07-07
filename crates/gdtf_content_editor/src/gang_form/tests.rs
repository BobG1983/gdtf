//! GANG-mode form-model unit tests (GTW-636 C5): the draft's mutators, the one-shot
//! autoload lifecycle, the loader-schema projection, and the PURE halves of the save
//! path (file name / path resolution / serialize round-trip). The REAL folder-walk
//! round-trip (write into a `TempDir` assets root → the actual [`GangsFamily`] loader)
//! lives in `tests/gang_mode.rs`.

use gdtf_assets::{ContentFamily, serialize_ron_pretty};
use gdtf_battle_sim::{
    armor::ArmorName,
    ganger::{
        Aim, Cool, GangMember, GangName, GangRegistry, GangRoster, GangerName, Grit, Luck,
        Reflexes, Speed, Strength, Toughness,
    },
    weapon::WeaponName,
};
use gdtf_content_families::GangsFamily;

use super::{
    draft::GangDraft,
    save::{draft_to_roster, gang_file_name, gang_save_path_in},
};

/// A two-member roster fixture. Magnitudes are arbitrary fixture data (NOT pinned
/// shipped tuning): the tests assert values SURVIVE, never that they equal a shipped
/// number. The first member authors a `melee_weapon` — the field the retired in-game
/// editor's mirror model DROPPED on save — so the round-trip pins that it now survives.
fn fixture_roster() -> GangRoster {
    GangRoster::new([
        GangMember {
            name:         GangerName::new("Edited Alpha".to_owned()),
            speed:        Speed::new(2.5),
            aim:          Aim::new(3.0),
            strength:     Strength::new(4.0),
            toughness:    Toughness::new(11.0),
            reflexes:     Reflexes::new(3.5),
            cool:         Cool::new(6.0),
            grit:         Grit::new(18.0),
            luck:         Luck::new(1.0),
            armor:        ArmorName::new("flak_vest".to_owned()),
            weapon:       WeaponName::new("stub_pistol".to_owned()),
            melee_weapon: Some(WeaponName::new("chainsword".to_owned())),
        },
        GangMember {
            name:         GangerName::new("Edited Bravo".to_owned()),
            speed:        Speed::new(4.0),
            aim:          Aim::new(5.0),
            strength:     Strength::new(2.0),
            toughness:    Toughness::new(9.0),
            reflexes:     Reflexes::new(4.0),
            cool:         Cool::new(7.0),
            grit:         Grit::new(15.0),
            luck:         Luck::new(2.0),
            armor:        ArmorName::new("mesh_armor".to_owned()),
            weapon:       WeaponName::new("autogun".to_owned()),
            melee_weapon: None,
        },
    ])
}

/// The `OnEnter(Editing)` seed is pristine: empty, with the one-shot autoload PENDING;
/// loading a gang fills the form AND marks the autoload done (so a later frame never
/// re-seeds over the author's work).
#[test]
fn default_is_pristine_and_load_gang_fills_the_form() {
    let mut draft = GangDraft::default();
    assert!(
        draft.autoload_pending(),
        "the fresh seed must autoload once"
    );
    assert_eq!(draft.name(), "");
    assert!(draft.members().is_empty());

    let roster = fixture_roster();
    draft.load_gang(&GangName::new("edited_gang".to_owned()), &roster);
    assert!(!draft.autoload_pending(), "a loaded gang ends the autoload");
    assert_eq!(draft.name(), "edited_gang");
    assert_eq!(draft.members(), roster.members.as_slice());
}

/// "New gang" mints an empty form whose autoload is DONE — a deliberate new gang must
/// never be clobbered by the one-shot registry seed; `mark_autoloaded` covers the
/// empty-registry branch the same way.
#[test]
fn new_gang_and_mark_autoloaded_end_the_one_shot_seed() {
    let draft = GangDraft::new_gang();
    assert!(!draft.autoload_pending());
    assert_eq!(draft.name(), "");
    assert!(draft.members().is_empty());

    let mut pristine = GangDraft::default();
    pristine.mark_autoloaded();
    assert!(!pristine.autoload_pending());
}

/// "Add member" appends the default placeholder record (parity with the retired
/// editor's default member: placeholder name, zero attributes, empty keys, no authored
/// melee); "Remove" deletes exactly that slot and reports out-of-range as `false`.
#[test]
fn add_and_remove_member_own_the_list_structure() {
    let mut draft = GangDraft::new_gang();
    draft.add_member();
    assert_eq!(draft.members().len(), 1);
    let Some(member) = draft.members().first() else {
        return;
    };
    assert_eq!(member.name.as_str(), "New Member");
    assert_eq!(member.weapon.as_str(), "");
    assert_eq!(member.armor.as_str(), "");
    assert_eq!(member.melee_weapon, None);

    assert!(!draft.remove_member(5), "out of range removes nothing");
    assert!(draft.remove_member(0));
    assert!(draft.members().is_empty());
}

/// The GTW-429 identity round-trip through the GANG mode's projection: an edited draft
/// → [`draft_to_roster`] → serialize (the shared pretty-RON seam) → deserialize the way
/// `RonAsset<GangRoster>` does → key by the saved file's stem (the loader's `.gang`
/// strip) → reload into a fresh draft → structural equality, INCLUDING the authored
/// `melee_weapon` key the retired editor dropped. No pinned magnitudes — identity, not
/// a number lock.
#[test]
fn edited_gang_round_trips_through_the_loader_schema() {
    let mut edited = GangDraft::new_gang();
    edited.load_gang(&GangName::new("edited_gang".to_owned()), &fixture_roster());

    let (name, roster) = draft_to_roster(&edited);
    let serialized = serialize_ron_pretty(&roster);
    assert!(
        serialized.is_ok(),
        "serializing the edited roster must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded_roster = ron::de::from_str::<GangRoster>(&serialized);
    assert!(
        reloaded_roster.is_ok(),
        "the serialized gang must round-trip through the GangRoster deserializer: {:?}",
        reloaded_roster.as_ref().err(),
    );
    let Ok(reloaded_roster) = reloaded_roster else {
        return;
    };

    // Key by the file STEM minus the `.gang` infix — the family seam's stem keying.
    let file_name = gang_file_name(&name);
    let stem = file_name.strip_suffix(".ron").unwrap_or(&file_name);
    let key = stem.strip_suffix(".gang").unwrap_or(stem).to_owned();
    let registry = GangRegistry::new([(GangName::new(key.clone()), reloaded_roster)]);

    let mut reloaded = GangDraft::new_gang();
    let roster_ref = registry.roster(&GangName::new(key));
    assert!(roster_ref.is_some(), "the reloaded registry holds the gang");
    let Some(roster_ref) = roster_ref else { return };
    reloaded.load_gang(&name, roster_ref);
    assert_eq!(
        reloaded, edited,
        "the reloaded gang must equal the edited draft (name + members + attributes + \
         weapon/armor/melee keys) — the GTW-415 round-trip",
    );
}

/// The save file name derives its compound suffix from [`GangsFamily::EXTENSION`] (the
/// one-owner extension discriminant — GTW-621), a path-hostile name sanitizes through
/// the shared seam, and an unnameable gang falls back to the documented `unnamed_gang`
/// stem; the resolved path lands under [`GangsFamily::FOLDER`].
#[test]
fn save_file_name_and_path_derive_from_the_one_owner_spellings() {
    let name = GangName::new("goliaths".to_owned());
    assert_eq!(
        gang_file_name(&name),
        format!("goliaths.{}", GangsFamily::EXTENSION),
    );

    let hostile = GangName::new("../../Evil Gang!".to_owned());
    assert_eq!(gang_file_name(&hostile), "evil_gang.gang.ron");
    let path = gang_save_path_in(std::path::Path::new("/tmp/root"), &hostile);
    let expected_tail = std::path::Path::new(GangsFamily::FOLDER).join("evil_gang.gang.ron");
    assert!(
        path.ends_with(&expected_tail),
        "the sanitized file lands under the gangs family folder: {path:?}",
    );

    let unnameable = GangName::new("!!!///".to_owned());
    assert_eq!(gang_file_name(&unnameable), "unnamed_gang.gang.ron");
}

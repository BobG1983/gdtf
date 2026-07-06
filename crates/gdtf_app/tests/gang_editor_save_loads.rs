//! GTW-621 regression pin: a gang SAVED through the editor's REAL write path
//! (`write_gang_roster_in` — the root-parameterized core the Save-button system's
//! wrapper calls) loads back into the [`GangRegistry`] through the REAL gangs
//! folder walk (the GTW-570 family seam over a live `AssetServer`), never an
//! in-memory mirror of the loader. The GTW-429 round-trip test deserialized a
//! string copy, so nothing pinned the on-disk extension the actual loader
//! dispatches on — this test does.
//!
//! RED on the pre-GTW-621 save convention (`<stem>.ron`, no `.gang` infix): the
//! bare `.ron` dispatches to the LAST-registered plain-`ron` loader (observed live
//! in the GTW-621 repro: `RonAsset<CombatLogTuning>`, whose whole-struct
//! `#[serde(default)]` shape parses ANY struct-shaped RON clean), the folder walk
//! SUCCEEDS, and the gangs family's `TypeId` filter silently skips the member —
//! the saved gang never reloads, with zero warns. GREEN once the written file name
//! derives its suffix from `GangsFamily::EXTENSION`.

#![cfg(debug_assertions)] // `write_gang_roster_in` rides the debug-gated editor save module.

use gdtf_app::test_support::{AppState, write_gang_roster_in};
use gdtf_battle_sim::{
    Aim, ArmorName, Cool, GangMember, GangName, GangRegistry, GangRoster, GangerName, Grit,
    Reflexes, Speed, Strength, Toughness, WeaponName, ganger::Luck,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a safety net against a genuine never-resolve
/// hang, never a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The saved gang's one-member roster fixture. The magnitudes are arbitrary
/// fixture data (NOT pinned shipped tuning): the test asserts the member
/// SURVIVES the disk round-trip, never that a value equals a shipped number.
fn saved_roster() -> GangRoster {
    GangRoster::new([GangMember {
        name:         GangerName::new("Saved Alpha".to_owned()),
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
        melee_weapon: None,
    }])
}

/// GTW-621 C4/A2 — the saved-gang round trip through the ACTUAL loader path: the
/// REAL editor write into a `TempDir` assets root, then the REAL `content/gangs`
/// folder walk over that root resolves a [`GangRegistry`] holding the saved gang,
/// keyed by its sanitized stem, with its member intact.
#[test]
fn saved_gang_loads_into_the_registry_through_the_real_folder_walk() {
    let Ok(assets_root) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    // SAVE through the real editor write path (the fn the Save-button wrapper
    // supplies the workspace root to), aimed at the isolated TempDir root.
    let name = GangName::new("edited_gang".to_owned());
    write_gang_roster_in(assets_root.path(), &name, &saved_roster());

    // Positive control: the write LANDED exactly one file under
    // `<root>/content/gangs/` — otherwise a missing gang below would mean a
    // failed write, not the extension drift under test.
    let written =
        std::fs::read_dir(assets_root.path().join("content/gangs")).map_or(0, Iterator::count);
    assert_eq!(
        written, 1,
        "the real save write must land exactly one file under content/gangs/",
    );

    // RELAUNCH: drive the real Load orchestration (kick-off → folder walk →
    // registry resolve) over the TempDir root. Every other content folder is
    // absent and fail-closes to its empty registry (the GTW-580 fixture
    // convention), so the gangs resolve is isolated.
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(assets_root.path().to_path_buf())
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<GangRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<GangRegistry>();
    assert!(
        registry.is_some(),
        "the gangs folder walk must resolve a GangRegistry within the safety-net budget",
    );
    let Some(registry) = registry else {
        return;
    };

    // The saved gang reloads, keyed by its (already-slug-shaped) stem …
    let reloaded = registry.roster(&name);
    assert!(
        reloaded.is_some(),
        "the saved gang `edited_gang` must reload into the GangRegistry through the real \
         folder walk — a missing key means the written extension drifted from the gangs \
         loader's registered extension (the GTW-621 bug); registry keys: {:?}",
        registry.keys().collect::<Vec<_>>(),
    );
    // … with its member intact (the disk round-trip preserved the roster).
    assert!(
        reloaded
            .and_then(|roster| roster.member(&GangerName::new("Saved Alpha".to_owned())))
            .is_some(),
        "the reloaded gang must still hold member \"Saved Alpha\"",
    );
}

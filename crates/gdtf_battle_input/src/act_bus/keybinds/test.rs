//! Tests for the keybind table + its loader (relocated from `keybinds.rs`, GTW-201).

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, Assets, Handle},
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};

use crate::keybinds::table::{BoundKey, Keybinds, KeybindsHandle, redrive_keybinds_on_asset_event};

/// AC7 — the shipped keybind RON deserializes through the generic
/// `ron::from_str` path the loader uses, and every declared act name resolves
/// to a `KeyCode`. Parsing the embedded file contents proves the schema +
/// the file agree (a malformed or incomplete file would fail here): serde
/// rejects a missing field, so a successful parse proves all six bound acts
/// are present and each resolves through [`BoundKey::key_code`].
///
/// It does NOT pin the six authored `KeyCode` MAGNITUDES — `keybinds.tuning.ron` is
/// editable, hot-swappable tuning data ("edit freely — every binding is data"),
/// so locking the file's chosen keys would be a brittle test on editable data
/// (the metric-constant exemption does not apply to keybinds). The non-brittle
/// INVARIANT we do assert is that the six bound keys are mutually distinct —
/// no two acts share a key, whatever the author binds them to. (The
/// `fire_mode_cycle` binding was REMOVED in GTW-254 — the popup picker replaced
/// the blind cycle.)
#[test]
fn shipped_keybinds_ron_deserializes_and_every_act_resolves() {
    // The exact bytes the loose `assets/core_tuning/keybinds.tuning.ron` ships, parsed the
    // same way `RonAssetLoader` parses them (`ron::de::from_bytes`).
    const RON: &str = include_str!("../../../../../assets/core_tuning/keybinds.tuning.ron");
    let parsed: Result<Keybinds, _> = ron::from_str(RON);
    assert!(
        parsed.is_ok(),
        "the shipped keybinds.tuning.ron must deserialize into Keybinds: {:?}",
        parsed.err(),
    );
    let Ok(binds) = parsed else { return };

    // Non-brittle invariant: the distinctly-keyed bound acts are mutually distinct (no two
    // collide on the same key), independent of which keys the author chose. GTW-458's
    // `select_next` / `select_prev` are DELIBERATELY the same key (Tab), differentiated by the
    // held Shift modifier (Tab = Next, Shift+Tab = Prev — one physical chord), so they are
    // EXCLUDED from this mutual-distinctness set; the dedicated check below pins their shared
    // binding instead.
    let bound = [
        binds.select_clear(),
        binds.level_up(),
        binds.level_down(),
        binds.stance_cycle(),
        binds.aim_toggle(),
        binds.facing_cycle(),
    ];
    for (i, lhs) in bound.iter().enumerate() {
        for rhs in &bound[i + 1..] {
            assert_ne!(
                lhs, rhs,
                "no two bound acts may share a key (shipped keybinds.tuning.ron has a collision)",
            );
        }
    }

    // GTW-458 — the Prev/Next cycle is ONE chord: `select_next` and `select_prev` bind to the
    // SAME key, differentiated by Shift. The keyboard surface reads `select_next` + the Shift
    // modifier, so this shared binding is the authored intent (not a collision). It must ALSO
    // not collide with any other act key (else Tab would fire two acts at once).
    assert_eq!(
        binds.select_next(),
        binds.select_prev(),
        "select_next / select_prev are one Shift-modified chord — they bind to the same key",
    );
    for other in &bound {
        assert_ne!(
            *other,
            binds.select_next(),
            "the cycle key must not collide with another bound act (shipped keybinds.tuning.ron)",
        );
    }
}

/// Each `BoundKey` variant resolves to its documented `KeyCode` — the single
/// typed translation the systems rely on (no hardcoded literal elsewhere).
#[test]
fn bound_key_resolves_to_its_key_code() {
    assert_eq!(BoundKey::KeyEscape.key_code(), KeyCode::Escape);
    assert_eq!(BoundKey::KeyPageUp.key_code(), KeyCode::PageUp);
    assert_eq!(BoundKey::KeyPageDown.key_code(), KeyCode::PageDown);
    assert_eq!(BoundKey::KeyBracketLeft.key_code(), KeyCode::BracketLeft);
    assert_eq!(BoundKey::KeyBracketRight.key_code(), KeyCode::BracketRight);
    assert_eq!(BoundKey::KeyTab.key_code(), KeyCode::Tab);
}

/// A `Keybinds` with `select_clear` bound to `clear` (the rest arbitrary-but-fixed) — the
/// distinguishing field the hot-reload test compares.
fn keybinds_with_clear(clear: BoundKey) -> Keybinds {
    Keybinds {
        select_clear:     clear,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

/// A headless app with the REAL keybind hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
/// (so `Assets<RonAsset<Keybinds>>` and the `AssetEvent` message buffer exist), the
/// `Keybinds` RON loader registered, and the redrive system in `Update`. Drives the ACTUAL
/// production redrive (`redrive_keybinds_on_asset_event`, wired into the input plugin's
/// build), not a copy — the combat-tuning hot-reload test shape.
fn hot_reload_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset::<Keybinds>()
        .add_systems(Update, redrive_keybinds_on_asset_event);
    app
}

/// Add a `RonAsset<Keybinds>` to the collection and return its handle.
fn add_asset(app: &mut App, keybinds: Keybinds) -> Handle<RonAsset<Keybinds>> {
    app.world_mut()
        .resource_mut::<Assets<RonAsset<Keybinds>>>()
        .add(RonAsset::new(keybinds))
}

/// Overwrite the in-memory payload of an already-added keybind asset — exactly what the
/// real file-watcher does when the loose `.ron` on disk is re-read.
fn hot_edit(app: &mut App, handle: &Handle<RonAsset<Keybinds>>, keybinds: Keybinds) {
    let mut assets = app.world_mut().resource_mut::<Assets<RonAsset<Keybinds>>>();
    if let Some(mut asset) = assets.get_mut(handle) {
        **asset = keybinds;
    }
}

/// GTW-533 C3: a previously-non-reloading asset — the keybind table — now hot-reloads. On a
/// matching `Modified` for the active keybind handle, the redrive overwrites the resident
/// `Keybinds` from the UPDATED in-memory payload WITHOUT any restart — the real
/// watcher+reload path, standing in the file-watcher with an in-memory edit + a `Modified`.
///
/// Pin-discriminating: dropping the redrive leaves `Keybinds` on the OLD clear key; a wrong
/// id filter would reload on any handle.
#[test]
fn modified_event_reloads_keybinds() {
    let mut app = hot_reload_app();

    let baseline = keybinds_with_clear(BoundKey::KeyEscape);
    let handle = add_asset(&mut app, baseline);
    app.world_mut().insert_resource(baseline);
    app.world_mut()
        .insert_resource(KeybindsHandle::new(handle.clone()));

    // First update: no event, the resource is untouched.
    app.update();

    // Hot-edit the asset to a DISTINCT clear key, then fire a Modified.
    let edited = keybinds_with_clear(BoundKey::KeyQ);
    assert_ne!(edited, baseline, "precondition: the edit must differ");
    hot_edit(&mut app, &handle, edited);
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
    app.update();

    assert_eq!(
        app.world().get_resource::<Keybinds>(),
        Some(&edited),
        "a Modified for the active keybind handle must reload Keybinds to the edited value \
         WITHOUT a restart",
    );
}

/// GTW-533 C3: a `Modified` for a DIFFERENT asset id leaves `Keybinds` untouched — the
/// filter is on the ACTIVE handle id only.
#[test]
fn modified_event_for_other_id_does_not_reload_keybinds() {
    let mut app = hot_reload_app();

    let baseline = keybinds_with_clear(BoundKey::KeyEscape);
    let active = add_asset(&mut app, baseline);
    let other = add_asset(&mut app, keybinds_with_clear(BoundKey::KeyQ));
    app.world_mut().insert_resource(baseline);
    app.world_mut().insert_resource(KeybindsHandle::new(active));

    app.update();
    hot_edit(&mut app, &other, keybinds_with_clear(BoundKey::KeyE));
    app.world_mut()
        .write_message(AssetEvent::Modified { id: other.id() });
    app.update();

    assert_eq!(
        app.world().get_resource::<Keybinds>(),
        Some(&baseline),
        "a Modified for a non-active keybind id must NOT reload Keybinds",
    );
}

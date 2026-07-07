//! GTW-654 C4/A2: the INJURY mode's REAL round-trips for BOTH artifact kinds —
//! author an injury def AND a weighting table in the form models, save them
//! through the REAL root-parameterized writes (`write_injury_in` /
//! `write_weighting_in`) into a `TempDir` assets root (the GTW-555 pattern — the
//! shipped `assets/` tree is NEVER written), then boot the REAL editor app rooted
//! at that directory and assert the actual bespoke injuries folder walk loads
//! both back: the def structurally identical in the [`InjuryRegistry`] and the
//! weighting folded into the [`InjuryTables`] bucket (canonically sorted).
//!
//! Also pins the GTW-654 lifecycle riders: the editor reaches `Editing` with the
//! injuries gate pair present and the two state-scoped drafts seeded. Per the
//! loader-tests convention the fixture magnitudes are arbitrary (values must
//! SURVIVE — no shipped-tuning pins).

use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight, InjuryWeighting,
    },
    severity::Severity,
};
use gdtf_content_editor::{
    EditorState, InjuryDraft, MapEditorPlugin, WeightingDraft, draft_to_def, draft_to_weighting,
    write_injury_in, write_weighting_in,
};
use gdtf_test_utils::advance_until;

/// A generous frame cap: the async asset loads under parallel `cargo` contention
/// take a non-deterministic number of frames, so this is a SAFETY NET (not a
/// timing budget) — the test polls the `EditorState::Editing` SIGNAL.
const MAX_UPDATES: u32 = 10_000;

/// The saved injury's key (sanitizes to itself, so it is also the file stem).
const SAVED_KEY: &str = "tempdir_wound";

/// The real editor app rooted at an ARBITRARY assets directory (the
/// `armor_mode.rs` recipe): only `content/injuries/` is materialized by this
/// test, so every other family fails closed to its empty registry (the no-strand
/// guarantee) while the injuries walk loads the REAL saved files.
fn editor_app_with_asset_root(root: &Path) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            // Headless-test noise suppression (GTW-139): the deliberate
            // failure-path asset errors of the unmaterialized families stay quiet.
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            }),
    );
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); with no render backend some render-provided params cannot
    // validate. `warn` restores the skip-with-a-log behavior (the shared harness
    // precedent).
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drives the app until [`EditorState::Editing`], then a few settle frames so the
/// `OnEnter(Editing)` command flushes apply before the assertions read.
fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — the Load gate (including the \
         injuries pair) did not resolve or fall back",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// The edited injury the test authors through the REAL form-model mutators: a
/// re-keyed def whose fields are all distinct (so a swapped field cannot
/// round-trip) and whose effects list exercises the effects-list ADD path.
fn edited_injury() -> InjuryDraft {
    let mut draft = InjuryDraft::new_injury();
    draft.set_key(SAVED_KEY.to_owned());
    let ron = "(name: \"Tempdir Wound\", category: Leg, severity: Minor, \
               popup_text: \"TEMPDIR\", log_text: \"is tempdir-wounded\", \
               inspect_text: \"Tempdir Wound -- fixture\", \
               effects: [MovementCostMul(1.5), Modify(stat: Speed, amount: -2)])";
    let parsed = ron::de::from_str::<InjuryDef>(ron);
    assert!(parsed.is_ok(), "the fixture def must parse: {parsed:?}");
    if let Ok(def) = parsed {
        draft.load_injury(&InjuryName::new(SAVED_KEY.to_owned()), &def);
    }
    draft
}

/// The edited weighting the test authors through the REAL form-model mutators: the
/// Leg context table with one Minor row naming the saved def's key (the by-
/// construction resolving reference the weighting combos enforce in the UI).
fn edited_weighting() -> WeightingDraft {
    let mut draft = WeightingDraft::default();
    draft.load_category(InjuryCategory::Leg, &InjuryTables::default());
    draft
        .weighting_mut()
        .minor
        .push(gdtf_battle_sim::injuries::WeightedInjuryEntry::new(
            InjuryName::new(SAVED_KEY.to_owned()),
            InjuryWeight::new(3),
        ));
    draft
}

/// GTW-654 — create → save BOTH artifact kinds (the REAL writes into a `TempDir`
/// assets root) → load through the REAL bespoke injuries folder walk → the
/// registry holds the SAME def (keyed by stem) and the tables hold the authored
/// weighting row in its `(Leg, Minor)` bucket, with the `Editing` gate + the two
/// scoped drafts along for the ride.
#[test]
fn saved_injury_and_weighting_round_trip_through_the_real_injuries_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE the def through the real root-parameterized write (the category
    // subfolder derives from the def's own authored category — `leg/`).
    let injury_draft = edited_injury();
    let (key, def) = draft_to_def(&injury_draft);
    let written = write_injury_in(dir.path(), &key, &def);
    assert!(
        written.is_ok(),
        "the real injury write must succeed: {:?}",
        written.as_ref().err(),
    );

    // SAVE the weighting through the real root-parameterized write
    // (`weighting/leg.weighting.ron`).
    let weighting: InjuryWeighting = draft_to_weighting(&edited_weighting());
    let written = write_weighting_in(dir.path(), &weighting);
    assert!(
        written.is_ok(),
        "the real weighting write must succeed: {:?}",
        written.as_ref().err(),
    );

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    // The two state-scoped INJURY drafts seeded on entering Editing (bevy-traps #1
    // via the GTW-575 seam).
    assert!(
        world.get_resource::<InjuryDraft>().is_some(),
        "the InjuryDraft must be seeded OnEnter(Editing)",
    );
    assert!(
        world.get_resource::<WeightingDraft>().is_some(),
        "the WeightingDraft must be seeded OnEnter(Editing)",
    );

    // ARTIFACT KIND 1 — the def: the REAL folder walk keyed the saved file by its
    // stem (minus the `.injury` infix) and loaded the SAME record.
    let registry = world.get_resource::<InjuryRegistry>();
    assert!(registry.is_some(), "the InjuryRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.def(&InjuryName::new(SAVED_KEY.to_owned()));
    assert_eq!(
        reloaded,
        Some(&def),
        "the reloaded injury must equal the saved def (every field incl. both effect \
         rows) — the GTW-437 stem-key round-trip through the REAL loader",
    );

    // ARTIFACT KIND 2 — the weighting: the REAL table fold resolved the row's key
    // against the loaded registry into the `(Leg, Minor)` bucket.
    let tables = world.get_resource::<InjuryTables>();
    assert!(tables.is_some(), "the InjuryTables must resolve");
    let Some(tables) = tables else { return };
    let bucket = tables.table_for_category(InjuryCategory::Leg, Severity::Minor);
    assert!(
        bucket.is_some_and(|rows| {
            rows.iter()
                .any(|row| row.injury.as_str() == SAVED_KEY && *row.weight == *InjuryWeight::new(3))
        }),
        "the reloaded (Leg, Minor) bucket must hold the authored row — the weighting \
         round-trip through the REAL table fold; bucket: {bucket:?}",
    );
    // The unauthored buckets stay absent (nothing leaked across severities).
    assert!(
        tables
            .table_for_category(InjuryCategory::Leg, Severity::Critical)
            .is_none(),
        "an unauthored bucket must not materialize",
    );
}

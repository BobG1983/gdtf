//! author an injury def AND a weighting table in the form models, save them
use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::{advance_until, asset_plugin_at};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        DamageContext, InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight,
        InjuryWeighting,
    },
    severity::Severity,
};
use gdtf_editor::{
    EditorState, InjuryDraft, MapEditorPlugin, WeightingDraft, draft_to_def, draft_to_weighting,
    write_injury_in, write_weighting_in,
};

const SAVED_KEY: &str = "tempdir_wound";

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
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(asset_plugin_at(root)),
    );
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

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

fn edited_weighting() -> WeightingDraft {
    let mut draft = WeightingDraft::default();
    draft.load_table(
        InjuryCategory::Leg,
        gdtf_battle_sim::injuries::DamageContext::Ranged,
        &InjuryTables::default(),
    );
    draft
        .weighting_mut()
        .minor
        .push(gdtf_battle_sim::injuries::WeightedInjuryEntry::new(
            InjuryName::new(SAVED_KEY.to_owned()),
            InjuryWeight::new(3),
        ));
    draft
}

#[test]
fn saved_injury_and_weighting_round_trip_through_the_real_injuries_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let injury_draft = edited_injury();
    let (key, def) = draft_to_def(&injury_draft);
    let written = write_injury_in(dir.path(), &key, &def);
    assert!(
        written.is_ok(),
        "the real injury write must succeed: {:?}",
        written.as_ref().err(),
    );

    let weighting: InjuryWeighting = draft_to_weighting(&edited_weighting());
    let written = write_weighting_in(dir.path(), &weighting);
    assert!(
        written.is_ok(),
        "the real weighting write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<InjuryDraft>().is_some(),
        "the InjuryDraft must be seeded OnEnter(Editing)",
    );
    assert!(
        world.get_resource::<WeightingDraft>().is_some(),
        "the WeightingDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<InjuryRegistry>();
    assert!(registry.is_some(), "the InjuryRegistry must resolve");
    let Some(registry) = registry else { return };
    let reloaded = registry.def(&InjuryName::new(SAVED_KEY.to_owned()));
    assert_eq!(
        reloaded,
        Some(&def),
        "the reloaded injury must equal the saved def (every field incl. both effect \
         rows) — the stem-key round-trip through the REAL loader",
    );

    let tables = world.get_resource::<InjuryTables>();
    assert!(tables.is_some(), "the InjuryTables must resolve");
    let Some(tables) = tables else { return };
    let bucket =
        tables.table_for_category(InjuryCategory::Leg, DamageContext::Ranged, Severity::Minor);
    assert!(
        bucket.is_some_and(|rows| {
            rows.iter()
                .any(|row| row.injury.as_str() == SAVED_KEY && *row.weight == *InjuryWeight::new(3))
        }),
        "the reloaded (Leg, Minor) bucket must hold the authored row — the weighting \
         round-trip through the REAL table fold; bucket: {bucket:?}",
    );
    assert!(
        tables
            .table_for_category(
                InjuryCategory::Leg,
                DamageContext::Ranged,
                Severity::Critical
            )
            .is_none(),
        "an unauthored bucket must not materialize",
    );
}

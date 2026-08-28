use bevy::app::App;
use bevy_egui::{EguiGlobalSettings, EguiPlugin};
use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext};
use gdtf_content_editor::EditorMode;
use gdtf_qa_protocol::message::{ProtocolVersion, QaRequest};

use crate::{
    harness::{advance_to_editing, editor_app_listening},
    hello::assert_hello_ok,
    names::EDITOR_SET_MODE,
    rows::BucketRow,
    setup::{draft_weighting, editor_mode, injury_tables, live_rows, table_of},
    socket::{Client, run_editor},
    support::{TestError, TestResult},
};

// An editing app that draws its own shell, which is where the form syncs run.
fn drawing_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, port) = editor_app_listening()?;
    app.add_plugins(EguiPlugin::default());
    // The editor spawns the one camera holding the primary context, so nothing else may claim it.
    app.insert_resource(EguiGlobalSettings {
        auto_create_primary_context: false,
        ..EguiGlobalSettings::default()
    });
    advance_to_editing(&mut app);
    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}

#[test]
fn opening_the_injury_tab_seeds_the_weighting_draft_out_of_the_live_tables() -> TestResult {
    let (mut app, mut client) = drawing_app_and_client()?;

    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Injury)"))?;

    let open = editor_mode(&app)?;
    if open != EditorMode::Injury {
        return Err(
            format!("the Injury tab must be open before the shell seeds it, got {open:?}").into(),
        );
    }
    let seeded = table_of(&draft_weighting(&app)?);
    assert_eq!(
        seeded.category.to_category(),
        InjuryCategory::ALL[0],
        "opening the tab seeds the first category the form offers",
    );
    assert_eq!(
        seeded.context.to_context(),
        DamageContext::ALL[0],
        "opening the tab seeds the first damage source the form offers",
    );

    let tables = injury_tables(&app)?;
    for bucket in [BucketRow::Minor, BucketRow::Major, BucketRow::Critical] {
        assert_eq!(
            seeded.bucket(bucket),
            live_rows(
                &tables,
                InjuryCategory::ALL[0],
                DamageContext::ALL[0],
                bucket.to_severity(),
            ),
            "the seeded {bucket:?} bucket must be the live tables' own rows for that key, in \
             their own order",
        );
    }
    assert!(
        !(seeded.minor.is_empty() && seeded.major.is_empty() && seeded.critical.is_empty()),
        "the live tables hold rows for that key, so an all-empty draft means the tab opened \
         without seeding rather than agreeing with the tables: {seeded:?}",
    );
    Ok(())
}

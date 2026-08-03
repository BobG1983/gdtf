use std::{fs, path::Path, time::Duration};

use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_battle_presenter::PlaybackTuning;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActProvenance},
    ganger::Direction,
};

use super::harness::*;

#[test]
fn shipped_playback_tuning_loads_and_drives_the_cursor() {
    let path = Path::new(WORKSPACE_ASSETS_ROOT).join("core_tuning/playback.tuning.ron");
    let read = fs::read_to_string(&path);
    assert!(
        read.is_ok(),
        "shipped playback.tuning.ron must be readable at {path:?}: {:?}",
        read.as_ref().err(),
    );
    let Ok(bytes) = read else {
        unreachable!("readability asserted above");
    };
    let parsed: Result<PlaybackTuning, _> = ron::de::from_str(&bytes);
    assert!(
        parsed.is_ok(),
        "shipped playback.tuning.ron must parse into PlaybackTuning, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(tuning) = parsed else {
        unreachable!("parse asserted above");
    };

    let mut app = playback_app();
    app.insert_resource(tuning.clone());
    let shooter = spawn_ganger(&mut app, ground(0, 0), Direction::East);
    seed(&mut app);
    append(
        &mut app,
        shooter,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );

    let beat = *tuning.minor_seconds;

    step(&mut app, Duration::from_millis(1));
    assert_eq!(
        *shown(&app),
        1,
        "the shipped-tuning cursor shows the appended entry",
    );
    assert!(
        holding(&app),
        "the cursor holds for the parsed minor beat once the entry is shown",
    );

    step(&mut app, Duration::from_secs_f32(beat * 0.5));
    assert!(
        holding(&app),
        "the cursor still holds before the parsed minor beat ({beat}s) has elapsed",
    );

    step(&mut app, Duration::from_secs_f32(beat));
    assert!(
        !holding(&app),
        "the parsed minor beat ({beat}s) drives the hold to completion",
    );
}

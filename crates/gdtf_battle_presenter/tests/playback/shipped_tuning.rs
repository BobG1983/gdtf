//! GTW-758 — the SHIPPED playback-tuning RON loads and drives the cursor.
//!
//! The `pacing` tests drive the cursor on the compiled-in `PlaybackTuning::default()` table.
//! This one closes the ticket's "the timing source loads and drives the cursor" clause: it
//! reads the actual shipped `assets/core_tuning/playback.tuning.ron` off disk (the same file
//! the game's hot-RON chain loads), parses it into a `PlaybackTuning`, inserts THAT into the
//! cursor's app, and proves the cursor holds for exactly the parsed beat — so a malformed
//! edit, or a field-name drift between the RON and the struct, reddens the suite.
//!
//! VALUE-AGNOSTIC: the expected hold is read FROM the parsed table, never a pinned magnitude,
//! so retuning a dwell in the `.ron` never reddens this test (only a parse break or a
//! cursor that stops reading the table does).

use std::{fs, path::Path, time::Duration};

use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_battle_presenter::PlaybackTuning;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActProvenance},
    ganger::Direction,
};

use super::harness::*;

/// The shipped playback-tuning RON parses into a `PlaybackTuning`, and that parsed table is
/// what the cursor holds for — the real-path proof that the timing SOURCE loads and DRIVES
/// the cursor, not just the compiled-in default the `pacing` suite runs on.
#[test]
fn shipped_playback_tuning_loads_and_drives_the_cursor() {
    // 1. LOADS: read + parse the shipped file exactly as the asset chain does.
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

    // 2. DRIVES: insert the PARSED table (not the default) and confirm the cursor holds for
    //    exactly its `minor_seconds` beat. A `BleedStarted` deed holds for `minor_seconds`.
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

    // Show the entry: the cursor begins the parsed hold.
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

    // A SUB-beat step must not release: the parsed beat is real and longer than the step.
    step(&mut app, Duration::from_secs_f32(beat * 0.5));
    assert!(
        holding(&app),
        "the cursor still holds before the parsed minor beat ({beat}s) has elapsed",
    );

    // Stepping past the beat releases the hold: the PARSED value drove the hold to completion.
    step(&mut app, Duration::from_secs_f32(beat));
    assert!(
        !holding(&app),
        "the parsed minor beat ({beat}s) drives the hold to completion",
    );
}

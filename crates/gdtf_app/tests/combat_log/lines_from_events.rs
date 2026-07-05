//! Events become classified lines: movement, injury severity colour, turn boundary.

use bevy::{prelude::*, text::TextColor};
use gdtf_app::test_support::{CombatLogLine, CombatLogRoot};
use gdtf_battle_presenter::severity_color;
use gdtf_battle_sim::{
    BodyPart, Cell, GainedInjury, InjuryInflicted, InjuryName, InspectText, LogText,
    MovementOccurred, PopupText, Severity, TurnStarted,
};

use super::harness::*;

/// The (rendered `Text`, `TextColor`) of every combat-log line. The line spawns at `alpha 0`
/// (its fade-in ramps it on), so callers compare the COLOR alpha-agnostically (RGB only).
fn line_texts_and_colors(app: &mut App) -> Vec<(String, Color)> {
    let entities = all_with::<CombatLogLine>(app);
    entities
        .into_iter()
        .filter_map(|e| {
            let text = app.world().get::<Text>(e)?.as_str().to_owned();
            let color = app.world().get::<TextColor>(e)?.0;
            Some((text, color))
        })
        .collect()
}

/// Whether some combat-log line reads exactly `text` AND is drawn in `color` (RGB-only, since a
/// freshly appended line spawns transparent and fades in — the hue, not the alpha, is the signal).
fn has_log_line(lines: &[(String, Color)], text: &str, color: Color) -> bool {
    let want = color.to_srgba();
    lines.iter().any(|(t, c)| {
        let got = c.to_srgba();
        t == text
            && (got.red - want.red).abs() < 0.001
            && (got.green - want.green).abs() < 0.001
            && (got.blue - want.blue).abs() < 0.001
    })
}

// ---------------------------------------------------------------------------------
// Lines from events — the log gains lines whose text matches the classifier output.
// ---------------------------------------------------------------------------------

/// A `MovementOccurred` message makes the log gain one line reading
/// `"<name> moved <from> -> <to>"` — the resolved name + the shared classifier phrasing.
#[test]
fn a_movement_message_appends_a_line_with_the_classified_text() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    // The log container exists (spawned on BattleRunning enter).
    assert_eq!(
        all_with::<CombatLogRoot>(&mut app).len(),
        1,
        "the combat-log container is spawned in BattleRunning",
    );
    // No lines yet (no events drained).
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    app.world_mut().write_message(MovementOccurred::new(
        ganger,
        Cell::new(3, 4),
        Cell::new(3, 6),
    ));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        1,
        "one movement event => one log line, got {texts:?}"
    );
    assert_eq!(
        texts[0], "Vex moved (3, 4) -> (3, 6)",
        "the line carries the resolved name + the classified movement phrasing",
    );
}

/// GTW-439, QA-gap remediation — the REAL system path: a genuine `InjuryInflicted` MESSAGE
/// written to the live buffer drives the registered forwarder → appender seam (GTW-572) to APPEND one
/// combat-log line reading `"<name> <log_text>"` (the wounded target resolved to its
/// `GangerName`, the authored log clause as the predicate) in the severity-scaled wound amber
/// (`severity_color`). NOT the pure `classify_log_event` classifier (covered by its own unit
/// test) — this drives the actual `InjuryInflicted` forwarder + appender in a live battle and asserts
/// the appended line entity.
///
/// Pin-discriminating: it FAILS if the `InjuryInflicted` → `CombatLogEvent::InjuryInflicted`
/// arm were removed from the forwarder/classifier (no line would be appended, failing the content
/// assertion), and it FAILS if the line were drawn a flat (non-severity) color, since the
/// assertion pins the EXACT `severity_color(Critical)` swatch (RGB) — distinct from a milder
/// tier's swatch.
#[test]
fn an_injury_message_appends_a_line_in_the_severity_colour() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    // The log starts empty (no events drained yet).
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    // A REAL InjuryInflicted on the named ganger (the buffer is registered by the sim's acts
    // plugin in a live battle). The combat log reads only the target (→ LogName), log_text, and
    // severity; the gained ledger + popup / inspect texts are filler (they drive other surfaces).
    let name = InjuryName::new("Lost Eye".to_owned());
    app.world_mut().write_message(InjuryInflicted {
        target: ganger,
        gained: GainedInjury::new(
            name.clone(),
            BodyPart::Head,
            Severity::Critical,
            Vec::new(),
            InspectText::new("Lost Eye -- -2 Aim".to_owned()),
        ),
        name,
        part: BodyPart::Head,
        severity: Severity::Critical,
        popup_text: PopupText::new("LOST EYE".to_owned()),
        log_text: LogText::new("loses an eye".to_owned()),
        inspect_text: InspectText::new("Lost Eye -- -2 Aim".to_owned()),
    });
    app.update();

    // POSITIVE assertion: the registered drain appended one line reading "<name> <log_text>" in
    // the Critical severity_color (RGB-only — the line spawns transparent and fades in).
    let lines = line_texts_and_colors(&mut app);
    assert!(
        has_log_line(
            &lines,
            "Vex loses an eye",
            severity_color(Severity::Critical)
        ),
        "an InjuryInflicted must append a \"Vex loses an eye\" line in the Critical \
         severity_color, got {lines:?}",
    );
}

/// A `TurnStarted` message for the player's faction makes the log gain the `"— Player turn —"`
/// boundary line (the classifier compares `now_active` to the `PlayerFaction`).
#[test]
fn a_turn_message_appends_the_player_turn_boundary_line() {
    let mut app = battle_running_app();
    app.update();

    // The live battle resolves a PlayerFaction — drive a turn-start for it so the boundary reads
    // "Player". Assert it is present (a live battle always has it) before reading.
    let player = app
        .world()
        .get_resource::<gdtf_battle_sim::PlayerFaction>()
        .copied();
    assert!(
        player.is_some(),
        "the live battle must have a PlayerFaction resolved"
    );
    let player = player.unwrap_or_default();
    app.world_mut().write_message(TurnStarted::new(*player));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert!(
        texts.iter().any(|t| t == "— Player turn —"),
        "a player-faction turn boundary logs \"— Player turn —\", got {texts:?}",
    );
}

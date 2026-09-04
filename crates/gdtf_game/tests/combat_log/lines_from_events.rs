use bevy::{prelude::*, text::TextColor};
use gdtf_battle_presenter::severity_color;
use gdtf_battle_sim::{
    acts::{InjuryInflicted, MoveCompleted, MovementOccurred},
    armor::BodyPart,
    injuries::{GainedInjury, InjuryName, InspectText, LogText, PopupText},
    prelude::Cell,
    severity::Severity,
    turn::TurnStarted,
};
use gdtf_game::test_support::{CombatLogLine, CombatLogRoot};

use super::harness::*;

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

#[test]
fn a_completed_move_appends_a_line_with_the_classified_text() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    let at = a_lit_cell(&app);
    app.update();

    assert_eq!(
        all_with::<CombatLogRoot>(&mut app).len(),
        1,
        "the combat-log container is spawned in BattleRunning",
    );
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    play(&mut app, MoveCompleted::new(ganger, at));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        1,
        "one completed move => one log line, got {texts:?}",
    );
    assert!(
        texts[0].starts_with("Vex") && !texts[0].contains("->"),
        "the line carries the resolved name and the classified movement phrasing, with no \
         coordinates: {texts:?}",
    );
}

#[test]
fn two_moves_by_one_ganger_read_the_same_however_far_it_walked() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    let (first, second) = two_lit_cells(&app);
    app.update();

    play(&mut app, MoveCompleted::new(ganger, first));
    app.update();
    play(&mut app, MoveCompleted::new(ganger, second));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        2,
        "two completed moves append two lines, got {texts:?}",
    );
    assert_eq!(
        texts[0], texts[1],
        "the movement line names the walker and nothing else, so two moves by one ganger \
         read alike however far apart they ended: {texts:?}",
    );
}

#[test]
fn a_played_step_appends_no_line_of_its_own() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(
        &mut app,
        MovementOccurred::new(ganger, Cell::new(3, 4), Cell::new(3, 6)),
    );
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert!(
        texts.is_empty(),
        "the panel logs completed moves, not stepped cells, so a played step appends \
         nothing: {texts:?}",
    );
}

#[test]
fn a_three_cell_walk_appends_exactly_one_movement_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    let at = a_lit_cell(&app);
    app.update();

    for step in 0..3 {
        play(
            &mut app,
            MovementOccurred::new(ganger, Cell::new(0, step), Cell::new(0, step + 1)),
        );
    }
    play(&mut app, MoveCompleted::new(ganger, at));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        1,
        "a three-cell walk is one move, so it appends one line — got {count}: {texts:?}",
        count = texts.len(),
    );
}

#[test]
fn an_injury_message_appends_a_line_in_the_severity_colour() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    let name = InjuryName::new("Lost Eye".to_owned());
    play(
        &mut app,
        InjuryInflicted {
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
        },
    );
    app.update();

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

#[test]
fn a_turn_message_appends_the_player_turn_boundary_line() {
    let mut app = battle_running_app();
    app.update();

    let player = app
        .world()
        .get_resource::<gdtf_battle_sim::battle::PlayerFaction>()
        .copied();
    assert!(
        player.is_some(),
        "the live battle must have a PlayerFaction resolved"
    );
    let player = player.unwrap_or_default();
    play(&mut app, TurnStarted::new(*player));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert!(
        texts.iter().any(|t| t == "— Player turn —"),
        "a player-faction turn boundary logs \"— Player turn —\", got {texts:?}",
    );
}

use bevy::{platform::collections::HashSet, prelude::*};
use gdtf_app::test_support::CombatLogLine;
use gdtf_battle_presenter::ShownSquadVisibility;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct},
    acts::MeleeStruck,
    prelude::CellLevel,
    resolve_hit::HpDamage,
    suppression::SuppressionApplied,
    visibility::SquadVisibility,
};

use super::harness::*;

#[test]
fn an_act_by_a_ganger_the_screen_cannot_see_appends_no_line() {
    let mut app = battle_running_app();
    let hidden = spawn_hidden(&mut app, "Skar");
    let dark = a_dark_cell(&app);
    app.update();

    play(&mut app, SuppressionApplied::new(hidden, dark));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert!(
        texts.is_empty(),
        "an act on a cell the screen does not light appends no line at all — the panel \
         printed {texts:?}",
    );
    assert!(
        !texts.iter().any(|line| line.contains("Skar")),
        "the hidden ganger's name must never reach the panel: {texts:?}",
    );
}

#[test]
fn an_observable_act_by_a_ganger_the_screen_cannot_see_names_someone() {
    let mut app = battle_running_app();
    let hidden = spawn_hidden(&mut app, "Skar");
    let mine = spawn_named(&mut app, "Vex");
    app.update();

    play(&mut app, MeleeStruck::new(hidden, mine, HpDamage::new(4)));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        1,
        "a hit the screen watches land is reported, so it appends one line: {texts:?}",
    );
    assert!(
        texts[0].starts_with("Someone") && texts[0].contains("Vex"),
        "the attacker the screen cannot see is reported as `Someone`, and the target it can \
         see is named: {texts:?}",
    );
    assert!(
        !texts[0].contains("Skar"),
        "the hidden attacker's own name must not reach the panel: {texts:?}",
    );
}

// Append an act nothing has played, so the playback gate shuts and the fog shadow freezes.
fn hold_the_screen_behind_the_sim(app: &mut App, at: CellLevel) {
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("a running battle seeds the act log the presenter plays");
    };
    log.append(RecordedAct::new(
        Entity::PLACEHOLDER,
        ActProvenance::Clock,
        ActDeed::EnteredView { at },
        ActWitnesses::unseen(),
    ));
}

// Light `at` in the sim's own fog, leaving the screen's frozen shadow where it was.
fn light_in_the_sim_only(app: &mut App, at: CellLevel) {
    let lit: HashSet<CellLevel> = std::iter::once(at).collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(lit.clone(), lit));
}

fn lit_by_the_sim(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<SquadVisibility>()
        .is_some_and(|fog| *fog.is_cell_visible(&at))
}

fn lit_by_the_screen(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<ShownSquadVisibility>()
        .is_some_and(|fog| *fog.visibility().is_cell_visible(&at))
}

#[test]
fn a_line_follows_the_screens_fog_and_not_the_sims() {
    let mut app = battle_running_app();
    let hidden = spawn_hidden(&mut app, "Skar");
    let dark = a_dark_cell(&app);
    app.update();

    hold_the_screen_behind_the_sim(&mut app, dark);
    light_in_the_sim_only(&mut app, dark);
    play(&mut app, SuppressionApplied::new(hidden, dark));
    app.update();

    let sim = lit_by_the_sim(&app, dark);
    let screen = lit_by_the_screen(&app, dark);
    assert!(
        sim && !screen,
        "the case only says anything while the sim is ahead of the screen: {dark:?} lit by \
         the sim = {sim}, lit by the screen = {screen}",
    );

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert!(
        texts.is_empty(),
        "the panel is a view, so it reports the cell the screen has lit and not the one the \
         sim has — the panel printed {texts:?}",
    );
}

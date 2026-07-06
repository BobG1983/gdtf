//! AC1 selected-ganger stat block: numeric HP/TU, wounds, injuries, mutate-in-place, empty state.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{
    StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList, StatName, StatPortrait, StatTuBar,
    StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    armor::BodyPart,
    ganger::{GangerName, Hp, HpMax, TuMax},
    inflicted_wound::{InflictedWound, InflictedWounds},
    injuries::{GainedInjury, InflictedInjuries, InjuryName, InspectText},
    prelude::Tu,
    severity::Severity,
};
use gdtf_ui::Pip;

use super::harness::*;

/// Reads the rendered `Text` of the single entity carrying marker `M`.
fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// The fill PERCENT of the status panel's `ProgressBar` carrying track-marker `M`.
fn bar_fill_percent<M: Component>(app: &mut App) -> Option<f32> {
    let track = single_with::<M>(app)?;
    bar_fill_at(app, track)
}

/// The number of VISIBLE filled pips (background != the lost color, visibility not Hidden)
/// in the single pips row carrying marker `M`. We count visible pips whose visibility is not
/// Hidden — a discriminating proxy for `WoundsMax` shown / `Wounds` filled.
fn visible_pip_count<M: Component>(app: &mut App) -> usize {
    let Some(row) = single_with::<M>(app) else {
        return 0;
    };
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(row)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    kids.into_iter()
        .filter(|&kid| {
            app.world().get::<Pip>(kid).is_some()
                && app.world().get::<Visibility>(kid) != Some(&Visibility::Hidden)
        })
        .count()
}

// ---------------------------------------------------------------------------------
// AC1 — status panel renders the shared stat block; LifeState/Weapon GONE.
// ---------------------------------------------------------------------------------

/// AC1 — with a selected ganger, the stat block shows its name + a NON-zero TU/HP bar fill +
/// the right Wounds pips; and there is NO life-state / weapon text line (the removed lines).
#[test]
fn status_panel_renders_the_selected_ganger_stat_block() {
    let mut app = battle_running_app();
    spawn_and_select(&mut app, default_setup());
    app.update();

    let name = line_text::<StatName>(&mut app).unwrap_or_default();
    assert!(name.contains("Vex Harker"), "name title: {name}");

    let tu = bar_fill_percent::<StatTuBar>(&mut app).unwrap_or(0.0);
    assert!((tu - 70.0).abs() < 0.5, "TU bar = 7/10 = 70% (got {tu})");

    let hp = bar_fill_percent::<StatHpBar>(&mut app).unwrap_or(0.0);
    assert!((hp - 50.0).abs() < 0.5, "HP bar = 8/16 = 50% (got {hp})");

    // WoundsMax = 3 visible pips, Wounds = 2 filled (we assert the visible count == 3).
    assert_eq!(
        visible_pip_count::<StatWoundsPips>(&mut app),
        3,
        "WoundsMax (3) pips must be visible",
    );

    // The removed LifeState + Weapon lines: NO text line anywhere reads them.
    let texts: Vec<String> = {
        let mut q = app.world_mut().query::<&Text>();
        q.iter(app.world()).map(|t| t.as_str().to_owned()).collect()
    };
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("State:") || t.starts_with("Weapon:")),
        "the LifeState + Weapon lines must be GONE (texts: {texts:?})",
    );
}

/// GTW-310 — the HP bar and TU bar each carry a numeric `cur/max` label that reads the
/// seeded ganger's `Hp`/`HpMax` and `Tu`/`TuMax` exactly, and a value change MUTATES the
/// label in place (no respawn). The displayed number must equal current/max.
#[test]
fn stat_block_shows_numeric_hp_and_tu() {
    let mut app = battle_running_app();
    // default_setup: Tu 7/10, Hp 8/16.
    spawn_and_select(&mut app, default_setup());
    app.update();

    let tu = line_text::<StatTuLabel>(&mut app).unwrap_or_default();
    assert_eq!(tu, "7/10", "the TU label reads Tu/TuMax (got {tu})");
    let hp = line_text::<StatHpLabel>(&mut app).unwrap_or_default();
    assert_eq!(hp, "8/16", "the HP label reads Hp/HpMax (got {hp})");

    // A changed selection mutates the SAME label entities in place.
    let tu_label_before = single_with::<StatTuLabel>(&mut app);
    let hp_label_before = single_with::<StatHpLabel>(&mut app);
    let mut other = default_setup();
    other.name = GangerName::new("Alex Mercer".to_owned());
    other.tu = Tu::new(3);
    other.tu_max = TuMax::new(12);
    other.hp = Hp::new(5);
    other.hp_max = HpMax::new(20);
    spawn_and_select(&mut app, other);
    app.update();

    assert_eq!(
        single_with::<StatTuLabel>(&mut app),
        tu_label_before,
        "the TU label entity is stable across a selection change (mutate, no respawn)",
    );
    assert_eq!(
        single_with::<StatHpLabel>(&mut app),
        hp_label_before,
        "the HP label entity is stable across a selection change (mutate, no respawn)",
    );
    assert_eq!(
        line_text::<StatTuLabel>(&mut app).unwrap_or_default(),
        "3/12",
        "the TU label mutated to the new ganger's Tu/TuMax",
    );
    assert_eq!(
        line_text::<StatHpLabel>(&mut app).unwrap_or_default(),
        "5/20",
        "the HP label mutated to the new ganger's Hp/HpMax",
    );
}

/// AC1 — an unwounded ganger hides the wound-name list; a wounded ganger shows it with the
/// matching "{tier} — {location}" entry.
#[test]
fn wound_list_reflects_inflicted_wounds() {
    let mut app = battle_running_app();

    // Unwounded -> the list container is Hidden.
    spawn_and_select(&mut app, default_setup());
    app.update();
    let list = single_with::<StatWoundList>(&mut app);
    assert!(list.is_some(), "the stat block carries a wound list");
    if let Some(list) = list {
        assert_eq!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "an unwounded ganger hides the wound-name list",
        );
    }

    // Wounded -> the list is shown and a line reads the wound.
    let mut wounded = default_setup();
    wounded.name = GangerName::new("Alex Mercer".to_owned());
    wounded.inflicted = InflictedWounds::new(vec![InflictedWound::new(
        Severity::Minor,
        BodyPart::LeftArm,
    )]);
    spawn_and_select(&mut app, wounded);
    app.update();

    if let Some(list) = single_with::<StatWoundList>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "a wounded ganger shows the wound-name list",
        );
    }
    // Some wound line reads "Minor — Left Arm".
    let lines: Vec<String> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Text, With<StatWoundLine>>();
        q.iter(app.world()).map(|t| t.as_str().to_owned()).collect()
    };
    assert!(
        lines
            .iter()
            .any(|l| l.contains("Minor") && l.contains("Left Arm")),
        "a wound line must read the inflicted wound (lines: {lines:?})",
    );
}

/// GTW-439 (C3) — the injury-name list is driven by the DURABLE `InflictedInjuries` ledger
/// (the persistent per-ganger list), NOT the transient `InjuryInflicted` message: an
/// uninjured ganger HIDES the list; a ganger whose ledger carries a `GainedInjury` SHOWS it
/// with a line reading that injury's authored `inspect_text`. PIN-DISCRIMINATING — the line
/// must read the exact authored text (a list driven by the wrong source, or not driven at
/// all, fails the content + visibility asserts).
#[test]
fn injury_list_reflects_inflicted_injuries() {
    let mut app = battle_running_app();

    // No injuries -> the list container is Hidden.
    spawn_and_select(&mut app, default_setup());
    app.update();
    let list = single_with::<StatInjuryList>(&mut app);
    assert!(list.is_some(), "the stat block carries an injury list");
    if let Some(list) = list {
        assert_eq!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "an uninjured ganger hides the injury-name list",
        );
    }

    // Injured -> the list is shown and a line reads the durable inspect_text.
    let mut injured = default_setup();
    injured.name = GangerName::new("Alex Mercer".to_owned());
    let mut ledger = InflictedInjuries::default();
    ledger.gain(GainedInjury::new(
        InjuryName::new("Lost Eye".to_owned()),
        BodyPart::Head,
        Severity::Critical,
        Vec::new(),
        InspectText::new("Lost Eye -- -2 Aim, -1 Cool".to_owned()),
    ));
    injured.injuries = ledger;
    spawn_and_select(&mut app, injured);
    app.update();

    if let Some(list) = single_with::<StatInjuryList>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "an injured ganger shows the injury-name list",
        );
    }
    // Some injury line reads the authored inspect_text verbatim (the persistent ledger source).
    let lines: Vec<String> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Text, With<StatInjuryLine>>();
        q.iter(app.world()).map(|t| t.as_str().to_owned()).collect()
    };
    assert!(
        lines.iter().any(|l| l == "Lost Eye -- -2 Aim, -1 Cool"),
        "an injury line must read the gained injury's authored inspect_text (lines: {lines:?})",
    );
}

/// AC1 — selecting a DIFFERENT ganger MUTATES the same stat block (stable entity ids — the
/// portrait/name nodes are the SAME entities, their content changes).
#[test]
fn selection_change_mutates_in_place() {
    let mut app = battle_running_app();
    spawn_and_select(&mut app, default_setup());
    app.update();
    let portrait_before = single_with::<StatPortrait>(&mut app);
    let name_before = single_with::<StatName>(&mut app);

    let mut other = default_setup();
    other.name = GangerName::new("Alex Mercer".to_owned());
    other.tu = Tu::new(2);
    spawn_and_select(&mut app, other);
    app.update();

    assert_eq!(
        single_with::<StatPortrait>(&mut app),
        portrait_before,
        "the portrait node entity is stable across a selection change (mutate, no respawn)",
    );
    assert_eq!(
        single_with::<StatName>(&mut app),
        name_before,
        "the name node entity is stable across a selection change",
    );
    let name = line_text::<StatName>(&mut app).unwrap_or_default();
    assert!(name.contains("Alex Mercer"), "the name mutated: {name}");
}

/// AC4-parity — no selection shows the empty state (name = "No ganger selected", bars empty)
/// and never stale data.
#[test]
fn no_selection_shows_empty_state() {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    let name = line_text::<StatName>(&mut app).unwrap_or_default();
    assert!(name.contains("No ganger selected"), "empty state: {name}");
    let tu = bar_fill_percent::<StatTuBar>(&mut app).unwrap_or(-1.0);
    assert!(
        tu.abs() < 0.5,
        "the TU bar is empty (0%) with no selection (got {tu})"
    );
}

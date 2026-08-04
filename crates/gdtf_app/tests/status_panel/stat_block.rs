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

fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

fn bar_fill_percent<M: Component>(app: &mut App) -> Option<f32> {
    let track = single_with::<M>(app)?;
    bar_fill_at(app, track)
}

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

    assert_eq!(
        visible_pip_count::<StatWoundsPips>(&mut app),
        3,
        "WoundsMax (3) pips must be visible",
    );

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

#[test]
fn stat_block_shows_numeric_hp_and_tu() {
    let mut app = battle_running_app();
    spawn_and_select(&mut app, default_setup());
    app.update();

    let tu = line_text::<StatTuLabel>(&mut app).unwrap_or_default();
    assert_eq!(tu, "7/10", "the TU label reads Tu/TuMax (got {tu})");
    let hp = line_text::<StatHpLabel>(&mut app).unwrap_or_default();
    assert_eq!(hp, "8/16", "the HP label reads Hp/HpMax (got {hp})");

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

#[test]
fn wound_list_reflects_inflicted_wounds() {
    let mut app = battle_running_app();

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

#[test]
fn injury_list_reflects_inflicted_injuries() {
    let mut app = battle_running_app();

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

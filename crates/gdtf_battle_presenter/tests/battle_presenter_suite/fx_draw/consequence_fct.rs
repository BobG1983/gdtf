use bevy::transform::components::Transform;
use gdtf_battle_presenter::{FctValence, FloatingCombatText, cell_to_world, valence_color};
use gdtf_battle_sim::{
    armor::BodyPart,
    armor_wear::ArmorBroken,
    effects::{
        bleed::Bleeding,
        dot::DotTicked,
        fields::{FieldDamage, FieldTicked},
    },
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    suppression::SuppressionApplied,
    weapon::DotDamage,
};

use super::{harness::*, probes::*};

#[test]
fn bleeding_pops_the_amber_bleeding_fct_tag() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(4, 7);
    let level = Level::new(0);
    let ganger = wounded_ganger(&mut app, cell, level, 2);

    play(&mut app, Bleeding::new(ganger));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "Bleeding", valence_color(FctValence::Status)),
        "a Bleeding consequence must pop an AMBER \"Bleeding\" tag, got {pops:?}",
    );

    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the Bleeding pop must anchor at the bleeding ganger's cell x ({})",
        anchor.x,
    );
}

#[test]
fn armor_broken_pops_the_red_armor_broken_fct_tag() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(9, 3);
    let level = Level::new(0);
    let ganger = wounded_ganger(&mut app, cell, level, 5);

    play(&mut app, ArmorBroken::new(ganger, BodyPart::Torso));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "Armor Broken", valence_color(FctValence::Damage)),
        "an ArmorBroken consequence must pop a RED \"Armor Broken\" tag, got {pops:?}",
    );
    assert!(
        !pops.iter().any(|(t, _)| t.starts_with("Armor -")),
        "no numeric \"Armor -N\" pop is built this slice (no integrity delta), got {pops:?}",
    );
}

#[test]
fn suppression_applied_pops_the_suppressed_fct_tag_at_the_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(11, 4);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let pinned = wounded_ganger(&mut app, cell, level, 2);

    play(&mut app, SuppressionApplied::new(pinned, at));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "a SuppressionApplied must pop a cowed \"SUPPRESSED\" tag, got {pops:?}",
    );

    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the SUPPRESSED pop must anchor at the pinned cell x ({})",
        anchor.x,
    );
}

#[test]
fn dot_ticked_pops_the_toxic_minus_amount_fct_tag_at_the_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(7, 9);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let ganger = wounded_ganger(&mut app, cell, level, 3);

    play(&mut app, DotTicked::new(ganger, at, DotDamage::new(4)));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "-4", valence_color(FctValence::Dot)),
        "a DotTicked must pop a toxic \"-4\" tag in the Dot valence, got {pops:?}",
    );

    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the DOT tick pop must anchor at the afflicted ganger's cell x ({})",
        anchor.x,
    );
}

#[test]
fn field_ticked_pops_the_hazard_minus_amount_fct_tag_at_the_cell() {
    let mut app = headless_renderer_app();
    app.add_message::<FieldTicked>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(8, 5);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let occupant = wounded_ganger(&mut app, cell, level, 2);

    play(
        &mut app,
        FieldTicked::new(occupant, at, FieldDamage::new(3)),
    );
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "-3", valence_color(FctValence::Field)),
        "a FieldTicked must pop a hazard \"-3\" tag in the Field valence, got {pops:?}",
    );

    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the field tick pop must anchor at the field cell x ({})",
        anchor.x,
    );
}

#[test]
fn a_raw_unplayed_consequence_signal_pops_nothing() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(11, 4);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let pinned = wounded_ganger(&mut app, cell, level, 2);

    let written = app
        .world_mut()
        .write_message(SuppressionApplied::new(pinned, at))
        .is_some();
    assert!(
        written,
        "the raw SuppressionApplied buffer must exist for this guard to mean anything",
    );
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        !has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "a consequence the cursor has NOT played must pop nothing, got {pops:?}",
    );

    play(&mut app, SuppressionApplied::new(pinned, at));
    app.update();
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "the PLAYED fact must still pop its tag, got {pops:?}",
    );
}

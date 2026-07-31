//! Consequence-family FCT tag pops: bleeding / armor / suppression / DOT / field
//! tick (GTW-302 s4, 526, 544, 545), each driven through the PACED buffer the family
//! reader drains (`Played<M>` — GTW-889), plus the regression guard that the RAW sim
//! message pops nothing.

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

/// GTW-302 (slice 4) — the REAL dispatch path: a `Bleeding { ganger }` drives the registered
/// `read_consequence_fct::<BleedingFct>` family reader (GTW-572) to spawn the AMBER `"Bleeding"` floating-combat-text pop over
/// the bleeding ganger's cell (ALONGSIDE the existing `read_bleeding` blood flash). Pins the
/// pop's text + valence on the registered-system path.
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

    // The pop is anchored at the bleeding ganger's cell (planar x — the FCT z is the Highlight
    // band, distinct from the cell z).
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

/// GTW-302 (slice 4) — the REAL dispatch path: an `ArmorBroken { ganger, part }` drives the
/// registered `read_consequence_fct::<ArmorBrokenFct>` family reader (GTW-572) to spawn the RED `"Armor Broken"` floating-combat-
/// text pop over the ganger's cell (ALONGSIDE the existing `read_armor_broken` spark flash).
/// Pins the destroy-crossing tag's text + RED valence; the numeric `"Armor -N"` is DEFERRED
/// (the message carries no integrity-delta amount), so NO `"Armor -"` pop appears.
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
    // The numeric "Armor -N" variant is DEFERRED (no integrity delta on the message) — no
    // "Armor -" pop should appear.
    assert!(
        !pops.iter().any(|(t, _)| t.starts_with("Armor -")),
        "no numeric \"Armor -N\" pop is built this slice (no integrity delta), got {pops:?}",
    );
}

/// GTW-526 C8 — the REAL dispatch path: a `SuppressionApplied { ganger, at }` drives the registered
/// `read_consequence_fct::<SuppressionFct>` family reader (GTW-572) to spawn the cowed `"SUPPRESSED"` floating-combat-text pop
/// over the pinned cell. Pins the pop's text + Suppressed valence + cell anchor on the
/// registered-system path (deleting the reader FAILS this).
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

    // POSITIVE: the system spawned a FloatingCombatText pop reading "SUPPRESSED" in the cowed
    // Suppressed valence (its OWN swatch, distinct from damage/wound/neutral/lethal).
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "a SuppressionApplied must pop a cowed \"SUPPRESSED\" tag, got {pops:?}",
    );

    // The pop is anchored at the pinned cell (planar x — the FCT z is the Highlight band,
    // distinct from the cell z).
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

/// GTW-544 — the REAL dispatch path: a `DotTicked { ganger, at, amount }` drives the registered
/// `read_consequence_fct::<DotFct>` family reader (GTW-572) to spawn the toxic `"-N"` floating-combat-text pop over the afflicted
/// ganger's cell. Pins the pop's `-{amount}` text + Dot valence + cell anchor on the
/// registered-system path (deleting the reader FAILS this).
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

    // POSITIVE: the system spawned a FloatingCombatText pop reading "-4" (the drained HP) in the
    // toxic Dot valence (its OWN swatch, distinct from damage/wound/neutral/lethal/suppressed).
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "-4", valence_color(FctValence::Dot)),
        "a DotTicked must pop a toxic \"-4\" tag in the Dot valence, got {pops:?}",
    );

    // The pop is anchored at the afflicted ganger's cell (planar x — the FCT z is the Highlight
    // band, distinct from the cell z).
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

/// GTW-545 — the REAL dispatch path: a `FieldTicked { occupant, at, amount }` drives the registered
/// `read_consequence_fct::<FieldFct>` family reader (GTW-572) to spawn the hazard `"-N"` floating-combat-text pop over the field cell.
/// Pins the pop's `-{amount}` text + Field valence + cell anchor on the registered-system path
/// (deleting the reader FAILS this).
#[test]
fn field_ticked_pops_the_hazard_minus_amount_fct_tag_at_the_cell() {
    let mut app = headless_renderer_app();
    // GTW-572 C4: the presenter registrar no longer add_messages FieldTicked (the sim's acts
    // plugin registers it in a live battle); this focused harness adds the buffer itself so
    // the gated family reader runs (the InjuryInflicted precedent).
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

    // POSITIVE: the system spawned a FloatingCombatText pop reading "-3" (the drained HP) in the
    // hazard Field valence (its OWN swatch, distinct from damage/wound/neutral/lethal/suppressed/DOT).
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "-3", valence_color(FctValence::Field)),
        "a FieldTicked must pop a hazard \"-3\" tag in the Field valence, got {pops:?}",
    );

    // The pop is anchored at the field cell (planar x — the FCT z is the Highlight band, distinct
    // from the cell z).
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

/// GTW-889 — the regression guard: a RAW sim signal, written straight to its own buffer,
/// pops NOTHING.
///
/// This is the reported defect stated as a test. The sim resolves a whole exchange in one
/// tick, so a `SuppressionApplied` reaches its buffer long before the playback cursor has
/// shown the shots that caused it. While the family reader drained that raw buffer, the
/// `"SUPPRESSED"` tag appeared at the top of the enemy turn with no shot on screen — which
/// is exactly what was reported. The reader now drains `Played<SuppressionApplied>`, so the
/// raw write is inert and the pop can no longer precede its cause.
///
/// Pin-discriminating: reverting the reader to `MessageReader<C::Signal>` pops the tag here
/// and FAILS.
#[test]
fn a_raw_unplayed_consequence_signal_pops_nothing() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(11, 4);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let pinned = wounded_ganger(&mut app, cell, level, 2);

    // The SIM's own buffer — what the sim writes the instant it resolves the suppression.
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

    // And the same fact, PLAYED, does pop — so the guard above is about pacing, not about a
    // reader that stopped working.
    play(&mut app, SuppressionApplied::new(pinned, at));
    app.update();
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "the PLAYED fact must still pop its tag, got {pops:?}",
    );
}

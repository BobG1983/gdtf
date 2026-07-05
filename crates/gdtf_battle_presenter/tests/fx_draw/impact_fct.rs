//! Shot-report FCT classification at impact (GTW-302 s3 / GTW-327 s2).

use bevy::{ecs::message::Messages, transform::components::Transform};
use gdtf_battle_presenter::{
    FctValence, FloatingCombatText, cell_to_world, severity_color, valence_color,
};
use gdtf_battle_sim::{
    BattleInProgress, BodyPart, Cell, CellLevel, DamageType, HitReport, Level, LifeState, Position,
    Severity, ShotDir, ShotFired, ShotKind, SimPos,
};

use super::{harness::*, probes::*};

/// GTW-302 (slice 3) / GTW-327 (slice 2) — the REAL dispatch path: a `ShotFired` carrying a
/// damaging, lethal ganger-hit `HitReport` drives the firing pipeline to spawn the
/// floating-combat-text pops (HP number RED, wound AMBER, penetration verdict, DOWN/DEAD lethal
/// RED), anchored at the hit ganger's cell — now spawned at the shot's IMPACT (after the bolt
/// flies), not on the drain frame. Pin-discriminates each pop's text + color.
#[test]
fn shot_fired_with_a_lethal_hit_spawns_the_classified_fct_pops() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // A struck ganger carrying a Position (the FCT reader anchors the pops at its cell).
    let cell = Cell::new(6, 4);
    let level = Level::new(0);
    let struck = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();

    // A Critical, DEAD, penetrating torso hit dealing 9 HP.
    let report = ganger_hit_report(
        struck,
        BodyPart::Torso,
        9,
        6,
        Severity::Critical,
        LifeState::Dead,
    );
    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  cell,
        impact_level: level,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    // Drain the ShotFired (spawn the bolt) on a zero-delta frame, then fly it to its impact —
    // the pops are spawned at the IMPACT now (GTW-327), so a single drain frame is not enough.
    fire_with_zero_delta(&mut app);
    // A handful of generous steps flies the bolt the muzzle->cell distance to arrival + seeds the
    // impact (which spawns the pops). One 50ms step is shorter than the FCT lifetime, so they
    // are still alive when read.
    step_app(&mut app, std::time::Duration::from_millis(50), 8);

    let pops = fct_pops(&mut app);
    // HP number (RED), wound (Critical amber), penetration verdict (GREY "Armor pierced"), DEAD
    // (lethal RED) — four distinct pops.
    assert!(
        has_fct_pop(&pops, "-9", valence_color(FctValence::Damage)),
        "the 9-HP hit must pop a RED \"-9\", got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Torso Critical", severity_color(Severity::Critical)),
        "a Critical torso wound must pop \"Torso Critical\" in the Critical amber, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Armor pierced", valence_color(FctValence::Neutral)),
        "a penetrating hit must pop a GREY \"Armor pierced\", got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "DEAD", valence_color(FctValence::Lethal)),
        "a Dead outcome must pop a lethal-RED \"DEAD\", got {pops:?}",
    );

    // The pops are anchored at the hit ganger's cell (x/y of cell_to_world; the FCT z is the
    // Highlight band, distinct from the cell z, so compare the planar position).
    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the FCT pops must anchor at the hit ganger's cell x ({})",
        anchor.x,
    );
}

/// GTW-302 (slice 3) / GTW-327 (slice 2) — a clean MISS `ShotFired` (a non-connecting shot)
/// spawns NO floating-combat-text pop at all on the real registered-system dispatch path, EVEN
/// after its tracer flies to the impact: a missed shot gets no pop (per user feedback, there is
/// no "Miss" popup text). The miss still rides a (numberless) bolt to its impact, so flying it to
/// completion proves the impact-spawn path emits nothing for an empty-pop shot.
#[test]
fn shot_fired_clean_miss_pops_nothing() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(2, 9);
    let level = Level::new(0);
    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(0.0, 1.0, 0.0)),
        impact_cell:  cell,
        impact_level: level,
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       Some(HitReport::no_effect(ShotKind::Miss)),
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    // Drain (spawn the bolt) then fly it all the way to its impact — even the impact-spawn path
    // must emit no pop for a miss.
    fire_with_zero_delta(&mut app);
    step_app(&mut app, std::time::Duration::from_millis(50), 8);

    let pops = fct_pops(&mut app);
    assert!(
        pops.is_empty(),
        "a clean miss must spawn no FCT pop, got {pops:?}"
    );
}

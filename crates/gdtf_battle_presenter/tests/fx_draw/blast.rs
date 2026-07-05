//! Throw-resolved grenade blast through the impact pipeline (GTW-546).

use bevy::{
    app::App, ecs::message::Messages, sprite::Sprite, time::TimeUpdateStrategy,
    transform::components::Transform,
};
use gdtf_battle_presenter::{ShotProjectile, cell_to_world};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, DamageType, Level, acts::ThrowResolved};

use super::harness::*;

/// How many sprite entities sit at (approximately) the world point `at` — the impact
/// animation `animate_impact` seeds carries a `Sprite` + a `Transform` at the seeded point, so
/// counting sprites at the landing world position is the discriminating check that the blast
/// glyph drew there. Excludes `ShotProjectile` sprites (there is no bolt for a lob this slice,
/// but the exclusion keeps the count to impact/flash glyphs).
fn sprites_at(app: &mut App, at: bevy::math::Vec3) -> usize {
    let mut q = app.world_mut().query_filtered::<&Transform, (
        bevy::prelude::With<Sprite>,
        bevy::prelude::Without<ShotProjectile>,
    )>();
    q.iter(app.world())
        .filter(|transform| transform.translation.distance(at) < 0.01)
        .count()
}

/// GTW-546 (presenter blast FX) — a `ThrowResolved { at, damage }` drives the grenade BLAST FX
/// through the EXISTING impact pipeline: `read_throw_resolved` seeds a `PendingImpact` at the
/// landing cell, and the SAME `animate_impact` (registered in the plugin's FX band) plays the
/// grenade's damage-type expanding-shockwave impact strip there — emitting exactly one
/// `ShotImpactResolved` as it consumes the seed. This is the in-engine QA evidence (headless)
/// that the blast is OBSERVABLE at the landing: the sim's throw-resolved signal renders the same
/// `AoE` hit FX the fire path draws, with no new blast infrastructure.
///
/// A blast is a multi-ganger fan with no single per-shot verdict, so the seeded impact carries a
/// `PLACEHOLDER` shooter + a `None` report — the emitted `ShotImpactResolved` carries no verdict
/// and the combat log renders NO outcome line for it (GTW-559, no phantom miss).
/// Pin-discriminates that: exactly ONE impact resolves, its shooter
/// is the placeholder, its report is `None`, and a sprite drew at `cell_to_world(landing)`.
#[test]
fn throw_resolved_draws_the_blast_impact_at_the_landing_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(9, 5);
    let level = Level::new(0);
    let landing = CellLevel::new(cell, level);
    let landing_world = cell_to_world(cell, level);

    // No impact has resolved before the throw — a clean baseline.
    assert_eq!(
        step_counting_impacts(&mut app, std::time::Duration::from_millis(16), 1),
        0,
        "no impact may resolve before any throw",
    );
    assert_eq!(
        sprites_at(&mut app, landing_world),
        0,
        "no blast glyph may sit at the landing before the throw resolves",
    );

    // The sim's throw-resolved signal — the arc's landing cell + the grenade's damage type
    // (Blast, the concussion node). read_throw_resolved seeds the impact THIS update; the
    // spawn_scene seed materializes on the SpawnScene schedule and animate_impact consumes it
    // the NEXT update (the same one-update handoff a shot's arrived projectile uses).
    app.world_mut()
        .resource_mut::<Messages<ThrowResolved>>()
        .write(ThrowResolved::new(landing, DamageType::Blast));

    // Update 1: read_throw_resolved drains the signal + seeds the PendingImpact (materializes on
    // SpawnScene). Update 2: animate_impact consumes the seed, spawns the impact glyph, emits the
    // ShotImpactResolved. Drain the impact signal across the two updates.
    let resolved = step_counting_impacts(&mut app, std::time::Duration::from_millis(16), 2);
    assert_eq!(
        resolved, 1,
        "a ThrowResolved must drive EXACTLY one blast impact through the existing pipeline",
    );

    // The blast glyph drew at the landing cell (the expanding-shockwave impact sprite).
    assert!(
        sprites_at(&mut app, landing_world) >= 1,
        "the blast impact glyph must draw at cell_to_world(landing) (the AoE hit FX)",
    );
}

/// GTW-546 (presenter blast FX) — the emitted blast `ShotImpactResolved` carries a PLACEHOLDER
/// shooter + a `None` report, so the combat log renders NO outcome line for it (GTW-559, no
/// phantom miss): a blast's numbers / downs ride the per-ganger wound / injury / bleed FCT
/// signals, not this detonation moment. Pin-discriminates the seed's no-verdict contract.
#[test]
fn throw_resolved_blast_impact_carries_no_shot_verdict() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(3, 8);
    let level = Level::new(0);
    let landing = CellLevel::new(cell, level);

    app.world_mut()
        .resource_mut::<Messages<ThrowResolved>>()
        .write(ThrowResolved::new(landing, DamageType::Chem));

    // Update 1 seeds the impact; update 2 resolves it and emits the signal — drain it then.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(16),
        ));
    app.update();
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);

    let impacts = drain_impacts(&mut app);
    assert_eq!(
        impacts.len(),
        1,
        "the blast must emit exactly one ShotImpactResolved as its seed resolves",
    );
    let Some(impact) = impacts.first() else {
        return;
    };
    assert_eq!(
        impact.shooter,
        bevy::ecs::entity::Entity::PLACEHOLDER,
        "a blast has no single shooter verdict — its seed carries the PLACEHOLDER shooter",
    );
    assert!(
        impact.report.is_none(),
        "a blast's numbers ride the per-ganger wound/injury signals — the impact report is None \
         (no verdict; the combat log renders no outcome line for it, GTW-559)",
    );
}

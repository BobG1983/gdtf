//! Field-overlay draw + on-death BOOM marker + `LeaveField` end-to-end (GTW-545/547).

use bevy::{app::App, ecs::message::Messages, transform::components::Transform};
use gdtf_battle_presenter::{
    FctValence, FieldCellSprite, FloatingCombatText, cell_to_world, valence_color,
};
use gdtf_battle_sim::{
    effects::{
        fields::{FieldDamage, FieldDef, FieldDuration, FieldRegistry, ImmuneArmorTypes},
        on_death::OnDeathOccurred,
    },
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    weapon::DamageType,
};

use super::{harness::*, probes::*};

/// A permanent `Chem` field def (the toxic-waste-pool catalog shape) — the fixture the overlay
/// test seeds a live field from.
fn toxic_pool_def() -> FieldDef {
    FieldDef::new(
        FieldDamage::new(3),
        DamageType::Chem,
        ImmuneArmorTypes::new([]),
        FieldDuration::Permanent,
    )
}

/// The number of live `FieldCellSprite` overlay sprites currently VISIBLE (shown, not hidden) in
/// the world — the pooled hazard tiles the overlay draws for the active storey.
fn visible_field_sprites(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query::<(&FieldCellSprite, &bevy::prelude::Visibility)>();
    q.iter(app.world())
        .filter(|(_, visibility)| matches!(visibility, bevy::prelude::Visibility::Visible))
        .count()
}

/// GTW-545 — the REAL draw path: a seeded `FieldRegistry` (a toxic pool on the active storey)
/// drives the registered `draw_field_overlay` system to spawn ONE persistent, visible
/// `FieldCellSprite` hazard-wash tile at the field cell — so a seeded field is VISIBLE on the map.
/// Pin-discriminates on the registered-system path (deleting the overlay draw leaves zero sprites).
#[test]
fn a_seeded_field_registry_draws_a_visible_hazard_cell_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // Seed the sim's live field registry with one toxic pool on the active (L0) storey — the
    // presenter reads this AUTHORITATIVE resource one-way.
    let cell = Cell::new(10, 7);
    let level = Level::new(0);
    let mut registry = FieldRegistry::new();
    registry.spawn(CellLevel::new(cell, level), toxic_pool_def());
    app.world_mut().insert_resource(registry);
    app.update();

    // POSITIVE: the overlay drew exactly one visible hazard-wash sprite for the one fielded cell.
    assert_eq!(
        visible_field_sprites(&mut app),
        1,
        "a single seeded field must draw exactly one visible FieldCellSprite hazard tile",
    );

    // The hazard tile is anchored at the field cell (planar x — the field z is the Field band,
    // distinct from the cell z, so compare the planar position).
    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FieldCellSprite, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the field hazard tile must anchor at the field cell x ({})",
        anchor.x,
    );
}

/// GTW-547 — the REAL dispatch path: an `OnDeathOccurred { entity, at }` drives the registered
/// `read_consequence_fct::<OnDeathFct>` family reader (GTW-572) to spawn the bold `"BOOM"` blast marker over the death cell — the
/// presenter FLOURISH that closes the Explode visibility gap (the sim's direct RNG-free blast drain
/// rides no shot-impact FX nor attrition pop). Pins the marker's text + lethal valence + cell anchor
/// on the registered-system path (deleting the reader FAILS this). The `OnDeathOccurred` buffer is
/// registered idempotently by `TopDownRendererPlugin` (like `MeleeResolved` / `FallOccurred`), so a
/// presenter-only harness can write it.
#[test]
fn on_death_occurred_pops_the_lethal_boom_marker_at_the_cell() {
    let mut app = headless_renderer_app();
    // GTW-572 C4: the presenter registrar no longer add_messages OnDeathOccurred (the sim's
    // acts plugin registers it in a live battle); this focused harness adds it itself.
    app.add_message::<OnDeathOccurred>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(11, 4);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let ganger = wounded_ganger(&mut app, cell, level, 0);

    app.world_mut()
        .resource_mut::<Messages<OnDeathOccurred>>()
        .write(OnDeathOccurred::new(ganger, at));
    app.update();

    // POSITIVE: the system spawned a FloatingCombatText marker reading "BOOM" in the LETHAL
    // blood-red valence (a terminal-death signal, NOT an attrition tick).
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "BOOM", valence_color(FctValence::Lethal)),
        "an OnDeathOccurred must pop a \"BOOM\" marker in the Lethal valence, got {pops:?}",
    );

    // The marker is anchored at the death cell (planar x — the FCT z is the Highlight band).
    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the on-death marker must anchor at the death cell x ({})",
        anchor.x,
    );
}

/// GTW-547 — a COVER death carries `Entity::PLACEHOLDER` (cover is not an entity) but still a valid
/// cell: the on-death family reader must still pop the `"BOOM"` marker at that cell (a smashed volatile
/// crate's detonation is visible), never fail on the placeholder entity.
#[test]
fn a_cover_on_death_still_pops_the_marker_at_the_cover_cell() {
    let mut app = headless_renderer_app();
    // GTW-572 C4: the harness adds the OnDeathOccurred buffer itself (see above).
    app.add_message::<OnDeathOccurred>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(3, 12);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);

    // A cover death: OnDeathOccurred::cover uses Entity::PLACEHOLDER — no ganger entity needed.
    app.world_mut()
        .resource_mut::<Messages<OnDeathOccurred>>()
        .write(OnDeathOccurred::cover(at));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "BOOM", valence_color(FctValence::Lethal)),
        "a cover on-death (placeholder entity) must still pop a \"BOOM\" marker, got {pops:?}",
    );
}

/// GTW-547 — the `LeaveField` END-TO-END render confirmation: a `LeaveField` on-death effect fires
/// by (1) spawning its referenced field into the sim's live `FieldRegistry` (exactly what the sim's
/// `resolve_on_death` → `leave_field` does via `FieldRegistry::spawn`) and (2) emitting an
/// `OnDeathOccurred` at that cell. In ONE presenter update this drives BOTH existing systems with
/// ZERO new field-render infra: `draw_field_overlay` draws the persistent hazard tile (the field
/// rides the existing GTW-545 overlay), and the on-death family reader pops the transient blast marker
/// over it. This is the "on-death effects ride existing rendering" proof for `LeaveField`.
#[test]
fn a_leave_field_on_death_draws_the_field_overlay_and_the_marker() {
    let mut app = headless_renderer_app();
    // GTW-572 C4: the harness adds the OnDeathOccurred buffer itself (see above).
    app.add_message::<OnDeathOccurred>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(6, 9);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);

    // (1) The LeaveField result: the referenced field is now live in the sim registry (the presenter
    //     reads this AUTHORITATIVE resource one-way — the same state the sim's leave_field spawns).
    let mut registry = FieldRegistry::new();
    registry.spawn(at, toxic_pool_def());
    app.world_mut().insert_resource(registry);
    // (2) The on-death occurrence at the same (cover) cell — a smashed crate that left the field.
    app.world_mut()
        .resource_mut::<Messages<OnDeathOccurred>>()
        .write(OnDeathOccurred::cover(at));
    app.update();

    // The field rides the EXISTING overlay with no new infra: exactly one visible hazard tile.
    assert_eq!(
        visible_field_sprites(&mut app),
        1,
        "a LeaveField's field must draw via the existing GTW-545 overlay (one visible tile)",
    );
    // And the transient on-death marker pops over it.
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "BOOM", valence_color(FctValence::Lethal)),
        "the LeaveField on-death must also pop the \"BOOM\" marker, got {pops:?}",
    );
}

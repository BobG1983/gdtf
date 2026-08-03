use bevy::{app::App, transform::components::Transform};
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

fn toxic_pool_def() -> FieldDef {
    FieldDef::new(
        FieldDamage::new(3),
        DamageType::Chem,
        ImmuneArmorTypes::new([]),
        FieldDuration::Permanent,
    )
}

fn visible_field_sprites(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query::<(&FieldCellSprite, &bevy::prelude::Visibility)>();
    q.iter(app.world())
        .filter(|(_, visibility)| matches!(visibility, bevy::prelude::Visibility::Visible))
        .count()
}

#[test]
fn a_seeded_field_registry_draws_a_visible_hazard_cell_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(10, 7);
    let level = Level::new(0);
    let mut registry = FieldRegistry::new();
    registry.spawn(CellLevel::new(cell, level), toxic_pool_def());
    app.world_mut().insert_resource(registry);
    app.update();

    assert_eq!(
        visible_field_sprites(&mut app),
        1,
        "a single seeded field must draw exactly one visible FieldCellSprite hazard tile",
    );

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

#[test]
fn on_death_occurred_pops_the_lethal_boom_marker_at_the_cell() {
    let mut app = headless_renderer_app();
    app.add_message::<OnDeathOccurred>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(11, 4);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let ganger = wounded_ganger(&mut app, cell, level, 0);

    play(&mut app, OnDeathOccurred::new(ganger, at));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "BOOM", valence_color(FctValence::Lethal)),
        "an OnDeathOccurred must pop a \"BOOM\" marker in the Lethal valence, got {pops:?}",
    );

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

#[test]
fn a_cover_on_death_still_pops_the_marker_at_the_cover_cell() {
    let mut app = headless_renderer_app();
    app.add_message::<OnDeathOccurred>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(3, 12);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);

    play(&mut app, OnDeathOccurred::cover(at));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "BOOM", valence_color(FctValence::Lethal)),
        "a cover on-death (placeholder entity) must still pop a \"BOOM\" marker, got {pops:?}",
    );
}

#[test]
fn a_leave_field_on_death_draws_the_field_overlay_and_the_marker() {
    let mut app = headless_renderer_app();
    app.add_message::<OnDeathOccurred>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(6, 9);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);

    let mut registry = FieldRegistry::new();
    registry.spawn(at, toxic_pool_def());
    app.world_mut().insert_resource(registry);
    play(&mut app, OnDeathOccurred::cover(at));
    app.update();

    assert_eq!(
        visible_field_sprites(&mut app),
        1,
        "a LeaveField's field must draw via the field overlay (one visible tile)",
    );
    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "BOOM", valence_color(FctValence::Lethal)),
        "the LeaveField on-death must also pop the \"BOOM\" marker, got {pops:?}",
    );
}

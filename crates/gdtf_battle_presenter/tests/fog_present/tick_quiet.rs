use bevy::{
    app::Update,
    asset::{AssetEvent, Assets},
    ecs::message::MessageReader,
    prelude::{
        Deref, DerefMut, DetectChanges, IntoScheduleConfigs, Query, Ref, Res, ResMut, Resource,
        Visibility, With,
    },
};
use gdtf_battle_presenter::{GangerSprite, PresenterSystems, TerrainFogMaterial, TerrainSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level},
    test_support::SituationBuilder,
};

use super::harness::*;

#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct RedirtyCount(usize);

#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct StorePoked(bool);

#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct ModifiedCount(usize);

#[derive(Resource, Default)]
struct QuietProbe {
        terrain_redirtied: RedirtyCount,
        actors_redirtied:  RedirtyCount,
        store_poked:       StorePoked,
        modified_events:   ModifiedCount,
}

fn record_quiet_probe(
    terrain: Query<Ref<Visibility>, With<TerrainSprite>>,
    actors: Query<Ref<Visibility>, With<GangerSprite>>,
    materials: Res<Assets<TerrainFogMaterial>>,
    mut modified: MessageReader<AssetEvent<TerrainFogMaterial>>,
    mut probe: ResMut<QuietProbe>,
) {
    probe.terrain_redirtied = RedirtyCount(terrain.iter().filter(Ref::is_changed).count());
    probe.actors_redirtied = RedirtyCount(actors.iter().filter(Ref::is_changed).count());
    probe.store_poked = StorePoked(materials.is_changed());
    probe.modified_events = ModifiedCount(
        modified
            .read()
            .filter(|event| matches!(event, AssetEvent::Modified { .. }))
            .count(),
    );
}

#[test]
fn steady_frame_leaves_fog_visibility_and_material_ticks_untouched() {
    let mut app = headless_renderer_app();
    app.init_resource::<QuietProbe>();
    app.add_systems(Update, record_quiet_probe.after(PresenterSystems::Draw));
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_cell = CellLevel::new(Cell::new(5, 5), l0);
    let explored_cell = CellLevel::new(Cell::new(6, 6), l0);
    let enemy_cell = CellLevel::new(Cell::new(20, 20), l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_cell, 0, Direction::East))
        .with_ganger(ganger_at(enemy_cell, 1, Direction::West))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, player_cell),
        "the terrain field must have drawn",
    );
    let player_sim = sim_entity_at(&mut app, player_cell);
    let enemy_sim = sim_entity_at(&mut app, enemy_cell);
    assert!(
        settle_actor(&mut app, player_sim) && settle_actor(&mut app, enemy_sim),
        "both ganger sprites must have materialized",
    );

    set_fog(&mut app, &[player_cell], &[explored_cell]);
    app.update();
    app.update();

    app.update();
    let probe = app.world().resource::<QuietProbe>();
    assert_eq!(
        *probe.terrain_redirtied, 0,
        "a steady frame with unchanged fog must leave every terrain tile's Visibility \
         change ticks untouched (the pre-GTW-627 unconditional `*visibility` write \
         re-dirtied every tile every frame)",
    );
    assert_eq!(
        *probe.actors_redirtied, 0,
        "a steady frame with unchanged fog must leave every ganger sprite's Visibility \
         change ticks untouched (the resolver writes via set_if_neq)",
    );
    assert!(
        !*probe.store_poked,
        "a steady frame with unchanged fog must not poke the Assets<TerrainFogMaterial> \
         resource tick (knobs are compared via Assets::get before any get_mut)",
    );
    assert_eq!(
        *probe.modified_events, 0,
        "a steady frame with unchanged fog must emit zero AssetEvent::Modified — no \
         per-frame uniform re-upload (the pre-GTW-627 writer took get_mut per shown tile \
         per frame)",
    );
}

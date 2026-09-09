use bevy::prelude::{Entity, Visibility};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level, LifeState},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

fn set_live_life(app: &mut bevy::app::App, sim: Entity, life: LifeState) {
    app.world_mut().entity_mut(sim).insert(life);
}

#[test]
fn a_sim_killed_player_ganger_stays_shown_until_the_death_plays() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .player_faction(Faction::new(0))
        .slab_at(at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger must have spawned");
    assert!(
        settle_actor(&mut app, sim),
        "the ganger sprite must have materialized",
    );

    set_fog(&mut app, &[], &[]);
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, sim),
        Some(Visibility::Inherited),
        "a live player ganger is shown regardless of fog (the OwnSquad relation)",
    );

    let Some(sim_entity) = sim else { return };
    set_live_life(&mut app, sim_entity, LifeState::Dead);
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, sim),
        Some(Visibility::Inherited),
        "a ganger the SIM has killed stays shown until the cursor plays his death",
    );

    set_drawn_life(&mut app, sim_entity, LifeState::Dead);
    app.update();
    let still_mapped = app
        .world()
        .get_resource::<GangerSprites>()
        .map(|sprites| sprites.contains(sim_entity));
    assert_eq!(
        still_mapped,
        Some(false),
        "once the death is played the sprite is despawned and unmapped",
    );
}

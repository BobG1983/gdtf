use super::support::{BleedingOut, PLAYER, end_turn, life_of, live_app, wounds_of};
use crate::{
    acts::StabilizeDownedRequested,
    ganger::LifeState,
    injuries::BleedAfflicted,
    metric::{Cell, CellLevel, Level},
    test_support::GangerEntityBuilder,
};

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

#[test]
fn stabilize_halts_the_bleed_clock_of_a_real_spawned_downed_ganger() {
    let mut app = live_app();

    let target = GangerEntityBuilder::new()
        .at(ground(11, 10))
        .faction(PLAYER)
        .combat_vitals(1, 100)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut()
        .entity_mut(target)
        .insert(BleedAfflicted::new(5));

    let ally = GangerEntityBuilder::new()
        .at(ground(10, 10))
        .faction(PLAYER)
        .combat_vitals(50, 10)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());

    end_turn(&mut app);
    assert_eq!(
        life_of(&app, target),
        LifeState::Downed,
        "the injury-HP bleed downs the target for real (no hand-inserted Downed)",
    );

    let before = wounds_of(&app, target);
    end_turn(&mut app);
    let bleeding = wounds_of(&app, target);
    assert!(
        bleeding < before,
        "a real Downed ganger bleeds out (Wounds drop): {before} -> {bleeding}",
    );

    app.world_mut()
        .write_message(StabilizeDownedRequested::new(ally, target));
    app.update();
    let stabilized_at = wounds_of(&app, target);

    end_turn(&mut app);
    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, target),
        stabilized_at,
        "stabilize must halt the bleed clock of a REAL-spawned Downed ganger \
         (the flag model skipped it)",
    );
    assert_eq!(
        life_of(&app, target),
        LifeState::Downed,
        "a stabilized ganger stays Downed (the clock halts; it does not revive)",
    );
}

#[test]
fn a_damage_downed_ganger_gains_bleeding_out_from_the_live_system_and_stabilizes() {
    let mut app = live_app();

    let target = GangerEntityBuilder::new()
        .at(ground(11, 10))
        .faction(PLAYER)
        .combat_vitals(50, 10)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());
    let ally = GangerEntityBuilder::new()
        .at(ground(10, 10))
        .faction(PLAYER)
        .combat_vitals(50, 10)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());

    app.world_mut().entity_mut(target).insert(LifeState::Downed);
    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "no marker until the live system reacts (nothing hand-inserted it)",
    );

    app.update();
    assert!(
        app.world().get::<BleedingOut>(target).is_some(),
        "the live mark_downed_bleeding must reify the apply_hit down as BleedingOut — \
         unwiring/emptying it drops this (and the whole weapon/melee/fall bleed-out path)",
    );

    let before = wounds_of(&app, target);
    end_turn(&mut app);
    end_turn(&mut app);
    let bleeding = wounds_of(&app, target);
    assert!(
        bleeding < before,
        "a damage-downed ganger bleeds out via the live-reified condition: {before} -> {bleeding}",
    );

    app.world_mut()
        .write_message(StabilizeDownedRequested::new(ally, target));
    app.update();
    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "stabilize removes the BleedingOut condition",
    );
    let stabilized_at = wounds_of(&app, target);
    end_turn(&mut app);
    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, target),
        stabilized_at,
        "stabilize halts the clock of a damage-downed ganger",
    );
    assert_eq!(
        life_of(&app, target),
        LifeState::Downed,
        "a stabilized ganger stays Downed (the clock halts; it does not revive)",
    );
}

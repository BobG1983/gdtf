//! AC6 — the empty-input invariant: with NO `*Requested` emitted, every dispatch system
//! is inert and mutates nothing across several updates. AC7 — the dispatch systems
//! co-schedule with the occupancy systems in `SimSystems::Simulate`: the combined
//! schedule builds + runs without an ambiguity/access panic, and a fire that kills the
//! target frees the grid slot (`sync_dead_gangers`, in the same set, observes the
//! `LifeState` change).

use super::support::*;

#[test]
fn no_messages_means_no_mutation() {
    let (mut app, shooter, target) = fire_scenario();
    // Also a posture actor and a downed pair, so every dispatch system has a subject
    // it WOULD mutate if it ran spuriously.
    let posture_actor = app
        .world_mut()
        .spawn((
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::North),
            Aiming::new(false),
            Tu::new(60),
        ))
        .id();
    let downed_actor = spawn_downed_actor(app.world_mut(), 20, 20, 1);
    let downed_target = spawn_downed_target(app.world_mut(), 21, 20, 1);
    // A move-capable actor (Position/Tu/LifeState) so dispatch_move has a subject it
    // WOULD mutate (Position + Tu) if it ran spuriously.
    let move_actor = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(Cell::new(30, 30), Level::new(0))),
            Tu::new(80),
            LifeState::Alive,
        ))
        .id();

    // Snapshot every relevant component before any update.
    let snap = |app: &App| {
        (
            app.world().get::<Hp>(target).copied(),
            app.world().get::<Wounds>(target).copied(),
            app.world().get::<LifeState>(target).copied(),
            app.world().get::<Tu>(shooter).copied(),
            app.world().get::<Stance>(posture_actor).copied(),
            app.world().get::<Facing>(posture_actor).copied(),
            app.world().get::<Aiming>(posture_actor).copied(),
            app.world().get::<Tu>(posture_actor).copied(),
            app.world().get::<Stabilized>(downed_target).copied(),
            app.world().get::<LifeState>(downed_target).copied(),
            app.world().get::<LifeState>(downed_actor).copied(),
            // The move actor's Position + Tu, nested so the outer tuple stays within
            // the 12-element PartialEq/Debug tuple-arity ceiling.
            (
                app.world().get::<Position>(move_actor).copied(),
                app.world().get::<Tu>(move_actor).copied(),
            ),
        )
    };
    let before = snap(&app);

    // Run several updates writing NO messages.
    for _ in 0..3 {
        app.update();
    }

    assert_eq!(
        snap(&app),
        before,
        "with no `*Requested` emitted every dispatch system is inert — nothing mutates",
    );
}

#[test]
fn dispatch_and_occupancy_co_schedule_and_a_kill_frees_the_slot() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // BOTH plugins tag their systems into SimSystems::Simulate. OccupancyMaintenancePlugin
    // owns the set's configure_sets (E10.0); SimActsPlugin only `.in_set`s into it.
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app);

    // A shooter aiming at a LOW-HP in-line target so the volley downs/kills it.
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
    // Deliberately fragile target (1 HP, 1 Wound, no armor) so the shot finishes it —
    // a relation (it dies), never a pinned damage number. Armor lives on related piece
    // entities now (GTW-323); this bare target wears none, so the shot lands on flesh.
    let target = app.world_mut().spawn(target_bundle(1, 1)).id();
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
    // Place the target in the occupancy grid + give it a Position so the occupancy
    // move-sync writes its PrevSlot (the slot sync_dead_gangers later frees).
    app.world_mut()
        .entity_mut(target)
        .insert(Position::new(target_at));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(target_at, Some(target));
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }
    // A LOW cover band at the target cell does not block a HIGH occupant; insert a
    // benign entry so the aim point reads a band (mirrors fire.rs in-line geometry).
    if let Some(mut cover) = app.world_mut().get_resource_mut::<CoverLedger>() {
        cover.insert(
            target_at,
            CoverEntry::seeded(
                CoverHp::new(10),
                HeightBand::High,
                ArmorProtection::new(0),
                ArmorHardness::new(0),
            ),
        );
    }

    // First update: sync_moved_gangers reacts to the target's Changed<Position>,
    // writing its PrevSlot and marking the occupancy slot. The combined schedule
    // builds + runs with no ambiguity/access panic (AC7's co-schedule check).
    app.update();
    let occupied_before = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&target_at));
    assert_eq!(
        occupied_before,
        Some(target),
        "after initial sync the target occupies its slot",
    );

    // Now FIRE — the dispatch mutates the target's LifeState (the kill), and in a
    // FOLLOWING deterministic update sync_dead_gangers (same set) observes the
    // Changed<LifeState> and frees the slot.
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update(); // dispatch_fire mutates LifeState this update
    app.update(); // sync_dead_gangers observes the change next update

    let life_after = app.world().get::<LifeState>(target).copied();
    let slot_after = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&target_at));
    // The fire downed or killed the target (a relation — not Alive).
    assert!(
        !matches!(life_after, Some(LifeState::Alive)),
        "the fire must have downed/killed the fragile target, got {life_after:?}",
    );
    // sync_dead_gangers (co-scheduled in the same set) freed the slot the dispatch's
    // LifeState change vacated — proving the two systems compose.
    assert_eq!(
        slot_after, None,
        "the occupancy slot the dispatched kill vacated must be freed by the \
         co-scheduled occupancy system",
    );
}

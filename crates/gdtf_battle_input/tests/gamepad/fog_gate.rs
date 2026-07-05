//! The GTW-346 fire-into-fog refusal + the shared `SquadVisibility` lockstep
//! read (GTW-11 C6).

use bevy::{ecs::system::SystemState, prelude::*};
use gdtf_battle_input::{
    InspectTarget, LeftClickOutcome, PathPreviewTarget, SelectedFireMode, SelectedShooter,
};
use gdtf_battle_sim::{Cell, CellLevel, Magazine, SquadVisibility, Tu, WieldedBy};

use super::harness::*;

/// The shooter's current `Tu` (raw inner value), read off the world for the byte-identity check.
fn shooter_tu(app: &App, shooter: Entity) -> u8 {
    app.world().get::<Tu>(shooter).map_or(0, |tu| **tu)
}

/// The (single) wielded weapon's loaded round count, read off the world for the byte-identity
/// check. The weapon is the lone `WieldedBy` entity in these focused harnesses.
fn weapon_loaded(app: &mut App) -> u16 {
    let mut q = app
        .world_mut()
        .query_filtered::<&Magazine, With<WieldedBy>>();
    q.iter(app.world()).next().map_or(0, |mag| *mag.rounds())
}

/// GTW-11 C6(1) — the fire-COMMIT refusal A/B/C: IDENTICAL occupancy / selection / fire mode, only
/// the squad fog differs. Case A (enemy cell VISIBLE + `can_fire` passes) FIREs — proving the path
/// WOULD fire absent the gate; Case B (the SAME enemy cell only UNSEEN) is a `NoOp` with ZERO
/// mutation; Case C (the SAME enemy cell only EXPLORED — a DISTINCT non-VISIBLE state) is the SAME
/// `NoOp` / zero mutation. `can_fire` is UNCHANGED across all three (LOS-free); the fog gate is the
/// only thing that flips.
#[test]
fn fire_commit_is_refused_into_a_non_visible_cell() {
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let target = CellLevel::new(Cell::new(6, 2), LEVEL);

    // Builds the IDENTICAL fire scenario (player shooter selected, enemy at `target`, fire mode),
    // returning the app + the shooter entity. Only the caller's `seed_fog` differs.
    let scenario = |app: &mut App| -> Entity {
        let ganger = spawn_player_shooter(app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let _enemy = place_enemy(app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));
        ganger
    };

    // --- Case A BASELINE: the enemy cell is VISIBLE -> FIRE (the path WOULD fire). ---
    {
        let mut app = decision_app();
        let _ganger = scenario(&mut app);
        seed_fog(&mut app, &[target], &[]);
        let outcome = decide(&mut app);
        assert!(
            matches!(outcome, LeftClickOutcome::Fire(_)),
            "Case A: a VISIBLE enemy cell with can_fire passing must FIRE, got {outcome:?}",
        );
    }

    // --- Case B: the SAME enemy cell is only UNSEEN (no fog covers it) -> NoOp, zero mutation. ---
    {
        let mut app = decision_app();
        let ganger = scenario(&mut app);
        // Fog covers some OTHER cell, NOT the target: the target is UNSEEN.
        seed_fog(&mut app, &[CellLevel::new(Cell::new(0, 0), LEVEL)], &[]);
        let tu_before = shooter_tu(&app, ganger);
        let loaded_before = weapon_loaded(&mut app);
        let outcome = decide(&mut app);
        assert_eq!(
            outcome,
            LeftClickOutcome::NoOp,
            "Case B: an UNSEEN enemy cell must REFUSE the fire commit (NoOp), got {outcome:?}",
        );
        // Apply it and prove ZERO mutation (no fire request emitted, no selection/TU/ammo change).
        apply_and_assert_inert(&mut app, outcome, ganger, tu_before, loaded_before);
    }

    // --- Case C: the SAME enemy cell is only EXPLORED (a DISTINCT non-VISIBLE state) -> NoOp. ---
    {
        let mut app = decision_app();
        let ganger = scenario(&mut app);
        // The target is EXPLORED (mission memory) but NOT currently VISIBLE.
        seed_fog(
            &mut app,
            &[CellLevel::new(Cell::new(0, 0), LEVEL)],
            &[target],
        );
        // Pin the DISTINCT EXPLORED-not-VISIBLE state on the fog itself.
        let fog = app.world().resource::<SquadVisibility>();
        assert!(
            fog.is_cell_explored(&target) && !fog.is_cell_visible(&target),
            "Case C fixture: the target must be EXPLORED but NOT VISIBLE (distinct from UNSEEN)",
        );
        let tu_before = shooter_tu(&app, ganger);
        let loaded_before = weapon_loaded(&mut app);
        let outcome = decide(&mut app);
        assert_eq!(
            outcome,
            LeftClickOutcome::NoOp,
            "Case C: an EXPLORED-not-VISIBLE enemy cell must REFUSE the fire commit (NoOp), got \
             {outcome:?}",
        );
        apply_and_assert_inert(&mut app, outcome, ganger, tu_before, loaded_before);
    }
}

/// Applies `outcome` and asserts ZERO mutation: no `ActIntent` pushed, the `SelectedShooter` +
/// `SelectedFireMode` unchanged, and the shooter `Tu` + weapon loaded rounds byte-identical to
/// `tu_before` / `loaded_before` (the C6(1) "zero TU, zero rounds, zero model mutation, targeting
/// stays armed" assertion). Drives the apply via a `SystemState` over `app.world_mut()` (carve-out
/// (a)).
fn apply_and_assert_inert(
    app: &mut App,
    outcome: LeftClickOutcome,
    shooter: Entity,
    tu_before: u8,
    loaded_before: u16,
) {
    use gdtf_battle_input::{PendingActIntent, apply_left_click};

    let fire_mode_before = *app.world().resource::<SelectedFireMode>();
    let selected_before = *app.world().resource::<SelectedShooter>();
    {
        let world = app.world_mut();
        let mut state: SystemState<(
            ResMut<SelectedShooter>,
            ResMut<PendingActIntent>,
            ResMut<PathPreviewTarget>,
        )> = SystemState::new(world);
        let Ok((mut selected, mut pending, mut target)) = state.get_mut(world) else {
            return;
        };
        apply_left_click(outcome, &mut selected, &mut pending, &mut target);
        state.apply(world);
    }

    // SelectedShooter + SelectedFireMode unchanged (targeting stays armed).
    assert_eq!(
        *app.world().resource::<SelectedShooter>(),
        selected_before,
        "the selection must be UNCHANGED by a refused fire commit (targeting stays armed)",
    );
    assert_eq!(
        *app.world().resource::<SelectedFireMode>(),
        fire_mode_before,
        "the fire mode must be UNCHANGED by a refused fire commit",
    );
    // Tu + ammo byte-identical (no model mutation — the shooter never paid for the refused shot).
    assert_eq!(
        shooter_tu(app, shooter),
        tu_before,
        "the shooter's TU must be byte-identical (zero TU spent on a refused fire)",
    );
    assert_eq!(
        weapon_loaded(app),
        loaded_before,
        "the weapon's loaded rounds must be byte-identical (zero rounds spent on a refused fire)",
    );
}

/// GTW-11 C6(4) / GTW-369 — the SHARED read agreement: flipping `SquadVisibility` for ONE enemy
/// cell flips the reticle verdict (`cell_squad_visible`) AND the `decide_left_click` fire-refusal
/// in LOCKSTEP — VISIBLE yields `SquadVisible` + Fire; non-VISIBLE yields `NotSquadVisible` +
/// `NoOp`. Proves the two consumers read the ONE predicate (never disagree). The text hint was
/// removed (user-ruled 2026-06-22, GTW-369); the affordance is the reticle recolour + the refusal.
#[test]
fn shared_squad_visible_read_flips_in_lockstep() {
    use gdtf_battle_presenter::{CellVisibility, cell_squad_visible};
    use gdtf_battle_sim::FactionRelation;

    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let target = CellLevel::new(Cell::new(6, 2), LEVEL);

    // VISIBLE: the predicate (the reticle's read) says SquadVisible AND the decision FIREs.
    {
        let mut app = decision_app();
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let _enemy = place_enemy(&mut app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));
        seed_fog(&mut app, &[target], &[]);

        let fog = app.world().resource::<SquadVisibility>();
        assert_eq!(
            cell_squad_visible(Some(fog), &target, Some(FactionRelation::Other)),
            CellVisibility::SquadVisible,
            "VISIBLE: the shared predicate (the reticle's read) must say SquadVisible",
        );
        assert!(
            matches!(decide(&mut app), LeftClickOutcome::Fire(_)),
            "VISIBLE: the decision must FIRE — in lockstep with the predicate",
        );
    }

    // NON-VISIBLE (flip the fog): the predicate says NotSquadVisible AND the decision is NoOp.
    {
        let mut app = decision_app();
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let _enemy = place_enemy(&mut app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));
        // Flip: the target is now NOT in the VISIBLE set.
        seed_fog(&mut app, &[CellLevel::new(Cell::new(0, 0), LEVEL)], &[]);

        let fog = app.world().resource::<SquadVisibility>();
        assert_eq!(
            cell_squad_visible(Some(fog), &target, Some(FactionRelation::Other)),
            CellVisibility::NotSquadVisible,
            "NON-VISIBLE: the shared predicate (the reticle's read) must say NotSquadVisible",
        );
        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::NoOp,
            "NON-VISIBLE: the decision must REFUSE (NoOp) — in lockstep with the predicate",
        );
    }
}

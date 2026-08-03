use bevy::{ecs::system::SystemState, prelude::*};
use gdtf_battle_input::{
    InspectTarget, LeftClickOutcome, PathPreviewTarget, SelectedFireMode, SelectedShooter,
};
use gdtf_battle_sim::{
    magazine::Magazine,
    prelude::{Cell, CellLevel, Tu},
    visibility::SquadVisibility,
    weapon::WieldedBy,
};

use super::harness::*;

fn shooter_tu(app: &App, shooter: Entity) -> u8 {
    app.world().get::<Tu>(shooter).map_or(0, |tu| **tu)
}

fn weapon_loaded(app: &mut App) -> u16 {
    let mut q = app
        .world_mut()
        .query_filtered::<&Magazine, With<WieldedBy>>();
    q.iter(app.world()).next().map_or(0, |mag| *mag.rounds())
}

#[test]
fn fire_commit_is_refused_into_a_non_visible_cell() {
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let target = CellLevel::new(Cell::new(6, 2), LEVEL);

    let scenario = |app: &mut App| -> Entity {
        let ganger = spawn_player_shooter(app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let _enemy = place_enemy(app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));
        ganger
    };

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

    {
        let mut app = decision_app();
        let ganger = scenario(&mut app);
        seed_fog(&mut app, &[CellLevel::new(Cell::new(0, 0), LEVEL)], &[]);
        let tu_before = shooter_tu(&app, ganger);
        let loaded_before = weapon_loaded(&mut app);
        let outcome = decide(&mut app);
        assert_eq!(
            outcome,
            LeftClickOutcome::NoOp,
            "Case B: an UNSEEN enemy cell must REFUSE the fire commit (NoOp), got {outcome:?}",
        );
        apply_and_assert_inert(&mut app, outcome, ganger, tu_before, loaded_before);
    }

    {
        let mut app = decision_app();
        let ganger = scenario(&mut app);
        seed_fog(
            &mut app,
            &[CellLevel::new(Cell::new(0, 0), LEVEL)],
            &[target],
        );
        let fog = app.world().resource::<SquadVisibility>();
        assert!(
            *fog.is_cell_explored(&target) && !*fog.is_cell_visible(&target),
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
    assert_eq!(
        shooter_tu(app, shooter),
        tu_before,
        "the shooter's TU must be unchanged (zero TU spent on a refused fire)",
    );
    assert_eq!(
        weapon_loaded(app),
        loaded_before,
        "the weapon's loaded rounds must be unchanged (zero rounds spent on a refused fire)",
    );
}

#[test]
fn shared_squad_visible_read_flips_in_lockstep() {
    use gdtf_battle_presenter::{CellVisibility, cell_squad_visible};
    use gdtf_battle_sim::visibility::FactionRelation;

    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let target = CellLevel::new(Cell::new(6, 2), LEVEL);

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

    {
        let mut app = decision_app();
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let _enemy = place_enemy(&mut app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));
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

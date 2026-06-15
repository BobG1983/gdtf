use bevy::prelude::*;

use crate::{
    scenes::running::game::battlescape::battle_running::{
        resources::BattleRunningComplete, systems::*,
    },
    states::BattleScapeState,
};

pub(in crate::scenes) struct GameBattleScapeBattleRunningScenePlugin;

impl Plugin for GameBattleScapeBattleRunningScenePlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(
        OnEnter(BattleScapeState::BattleRunning),
        (print_on_enter, insert_turn_budget),
    )
    // The saturating per-tick decrement runs BEFORE the completion gate this same
    // FixedUpdate, so the marker can appear on the tick the budget reaches zero
    // (deterministic ordering — `bevy-traps.md` #3). The gate only inserts while the
    // marker is still absent, so it inserts exactly once.
    .add_systems(
        FixedUpdate,
        (
            decrement_turn_budget,
            game_battlescape_battle_running_complete
                .run_if(not(resource_exists::<BattleRunningComplete>)),
        )
            .chain()
            .run_if(in_state(BattleScapeState::BattleRunning)),
    )
    .add_systems(
        FixedUpdate,
        move_on.run_if(
            in_state(BattleScapeState::BattleRunning).and(resource_exists::<BattleRunningComplete>),
        ),
    )
    .add_systems(
        OnExit(BattleScapeState::BattleRunning),
        (print_on_exit, cleanup),
    );
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce as _, prelude::World};

    use crate::scenes::running::game::battlescape::battle_running::{
        resources::{BattleRunTurnBudget, BattleRunningComplete},
        systems::{
            cleanup, decrement_turn_budget, game_battlescape_battle_running_complete,
            insert_turn_budget,
        },
    };

    /// AC7 (discipline) + AC3: the budget's saturating decrement bottoms out at zero (never
    /// wraps below it) and `is_spent` flips exactly at zero — the gate's real, finite
    /// condition. A relation, not a pinned starting magnitude.
    #[test]
    fn budget_decrement_saturates_at_zero_and_marks_spent() {
        let mut budget = BattleRunTurnBudget::new(2);
        assert!(!budget.is_spent(), "a fresh non-zero budget is not spent");
        budget = budget.tick(); // 2 -> 1
        assert!(!budget.is_spent(), "one tick left is not yet spent");
        budget = budget.tick(); // 1 -> 0
        assert!(budget.is_spent(), "the budget is spent at zero");
        budget = budget.tick(); // 0 -> 0 (saturating, no wrap)
        assert_eq!(
            budget,
            BattleRunTurnBudget::new(0),
            "the decrement saturates at zero — it must never wrap below zero",
        );
    }

    /// AC2: with a budget still above zero, the completion gate does NOT insert the marker.
    #[test]
    fn complete_gate_does_not_fire_while_budget_remains() {
        let mut world = World::new();
        world.insert_resource(BattleRunTurnBudget::new(1));
        let result = world.run_system_once(game_battlescape_battle_running_complete);
        assert!(result.is_ok(), "the gate system must run");
        assert!(
            world.get_resource::<BattleRunningComplete>().is_none(),
            "the completion gate must NOT insert the marker while the budget remains (>0)",
        );
    }

    /// AC3: once the budget is spent (zero), the completion gate inserts `BattleRunningComplete`.
    #[test]
    fn complete_gate_fires_when_budget_spent() {
        let mut world = World::new();
        world.insert_resource(BattleRunTurnBudget::new(0));
        let result = world.run_system_once(game_battlescape_battle_running_complete);
        assert!(result.is_ok(), "the gate system must run");
        assert!(
            world.get_resource::<BattleRunningComplete>().is_some(),
            "the completion gate must insert the marker once the budget is spent (==0)",
        );
    }

    /// AC2/AC3: the per-tick decrement system saturatingly spends the budget on the real path,
    /// and the gate fires exactly the tick the budget reaches zero (the decrement-then-gate
    /// order the plugin chains).
    #[test]
    fn decrement_then_gate_completes_exactly_at_zero() {
        let mut world = World::new();
        world.insert_resource(BattleRunTurnBudget::new(2));

        // Tick 1: 2 -> 1, not spent -> no marker.
        assert!(world.run_system_once(decrement_turn_budget).is_ok());
        assert!(
            world
                .run_system_once(game_battlescape_battle_running_complete)
                .is_ok()
        );
        assert_eq!(
            world.get_resource::<BattleRunTurnBudget>().copied(),
            Some(BattleRunTurnBudget::new(1)),
            "after one tick the budget is decremented",
        );
        assert!(
            world.get_resource::<BattleRunningComplete>().is_none(),
            "the marker must not appear while the budget remains",
        );

        // Tick 2: 1 -> 0, spent -> marker inserted.
        assert!(world.run_system_once(decrement_turn_budget).is_ok());
        assert!(
            world
                .run_system_once(game_battlescape_battle_running_complete)
                .is_ok()
        );
        assert_eq!(
            world.get_resource::<BattleRunTurnBudget>().copied(),
            Some(BattleRunTurnBudget::new(0)),
            "the budget reaches zero",
        );
        assert!(
            world.get_resource::<BattleRunningComplete>().is_some(),
            "the marker is inserted exactly the tick the budget reaches zero",
        );
    }

    /// AC6: `OnExit(BattleRunning)` cleanup removes BOTH the marker AND the budget, and the
    /// `OnEnter` insert re-seeds a FRESH (non-zero) budget — so a re-entry starts ungated.
    #[test]
    fn cleanup_removes_both_resources_then_reentry_reseeds_fresh_budget() {
        let mut world = World::new();
        // Simulate a completed run: both the spent budget and the marker are present.
        world.insert_resource(BattleRunTurnBudget::new(0));
        world.insert_resource(BattleRunningComplete);

        // OnExit cleanup removes both.
        assert!(world.run_system_once(cleanup).is_ok(), "cleanup must run");
        assert!(
            world.get_resource::<BattleRunTurnBudget>().is_none(),
            "cleanup must remove the per-run turn budget",
        );
        assert!(
            world.get_resource::<BattleRunningComplete>().is_none(),
            "cleanup must remove the completion marker",
        );

        // A re-entry's OnEnter re-seeds a FRESH, non-zero budget (not the leftover zero).
        assert!(
            world.run_system_once(insert_turn_budget).is_ok(),
            "the OnEnter insert must run on re-entry",
        );
        let reseeded = world.get_resource::<BattleRunTurnBudget>().copied();
        assert!(
            reseeded.is_some_and(|budget| !budget.is_spent()),
            "re-entry must re-insert a FRESH (non-zero, ungated) budget, got {reseeded:?}",
        );
    }
}

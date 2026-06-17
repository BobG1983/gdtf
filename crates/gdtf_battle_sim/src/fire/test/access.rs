//! AC1 — the two-query design is access-compatible (no `B0001` conflict).

use super::support::*;

/// AC1 — the two-query design is access-compatible: a `SystemState` over
/// `(ShooterQuery, TargetQuery)` constructs WITHOUT a `B0001` conflict panic.
/// Building the `SystemState` validates the access set, so this IS the regression
/// check that the shooter (mut Tu/Magazine, no `LifeState`) and target (mut
/// Hp/Wounds/`LifeState`/`WornArmor`) queries share no conflicting mutable
/// component.
#[test]
fn two_queries_are_access_compatible_no_b0001() {
    let mut world = World::new();
    // If the two queries had a conflicting mutable access, SystemState::new would
    // panic here (the param-validation B0001 check). Reaching the assert proves
    // the design is disjoint.
    let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
    let (_shooters, _targets) = state.get_mut(&mut world);
    // Reaching here means the access set validated — the two queries coexist.
}

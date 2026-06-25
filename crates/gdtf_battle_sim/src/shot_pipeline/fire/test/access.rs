//! AC1 — the two-query design is access-compatible (no `B0001` conflict).

use super::support::*;

/// AC1 — the two-query design is access-compatible: a `SystemState` over
/// `(ShooterQuery, TargetQuery)` constructs WITHOUT a `B0001` conflict panic.
/// Building the `SystemState` validates the access set, so this IS the regression
/// check that the shooter (mut Tu, no `LifeState`) and target (mut
/// Hp/Wounds/`LifeState`) queries share no conflicting mutable component (the armor
/// integrity now wears on related piece entities, GTW-323).
#[test]
fn two_queries_are_access_compatible_no_b0001() {
    let mut world = World::new();
    // If the two queries had a conflicting mutable access, SystemState::new would
    // panic here (the param-validation B0001 check). Reaching the assert proves
    // the design is disjoint.
    let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
    // `get_mut` now returns a `Result` (Bevy 0.19); an `Ok` IS the disjoint-access
    // proof — a conflicting mutable access would surface as the validation error.
    let access = state.get_mut(&mut world);
    assert!(
        access.is_ok(),
        "the shooter + target queries must validate as access-compatible (no B0001)",
    );
}

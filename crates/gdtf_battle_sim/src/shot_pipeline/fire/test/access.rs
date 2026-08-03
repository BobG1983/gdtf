use super::support::*;

#[test]
fn two_queries_are_access_compatible_no_b0001() {
    let mut world = World::new();
    let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
    let access = state.get_mut(&mut world);
    assert!(
        access.is_ok(),
        "the shooter + target queries must validate as access-compatible (no B0001)",
    );
}

//! AC1 — the seven `*Requested` types are constructible, carry their actor ref(s) +
//! owned payload, and `FireRequested` / `MoveRequested` have no lifetime parameter.

use super::support::*;

/// Mint a valid [`Entity`] handle for a test fixture (no `from_raw` in 0.18) — spawn
/// an empty entity in a fresh world and take its id.
fn an_entity() -> Entity {
    World::new().spawn_empty().id()
}

#[test]
fn requested_types_construct_and_carry_their_refs_and_payload() {
    let shooter = an_entity();
    let target = an_entity();
    let mode = single_mode(0.2, 3);

    // FireRequested carries the shooter Entity + an OWNED FireModeSpec + Cell/Level
    // (no borrow → no lifetime parameter; that this compiles as `FireRequested`
    // without `<'_>` is the proof).
    let f = FireRequested::new(shooter, mode, Cell::new(8, 5), Level::new(0));
    assert_eq!(
        f.shooter, shooter,
        "FireRequested carries the shooter Entity"
    );
    assert_eq!(f.mode, mode, "FireRequested carries the OWNED FireModeSpec");
    assert_eq!(f.target_cell, Cell::new(8, 5), "carries the target Cell");
    assert_eq!(f.target_level, Level::new(0), "carries the target Level");

    // Posture requests carry one actor Entity + the requested kind.
    let a = SetAimingRequested::new(shooter, AimRequest::new(true));
    assert_eq!(a.actor, shooter);
    assert!(*a.aim, "the requested aim flag is carried");
    let s = SetStanceRequested::new(shooter, StanceKind::Prone);
    assert_eq!(s.actor, shooter);
    assert_eq!(s.stance, StanceKind::Prone);
    let fa = SetFacingRequested::new(shooter, Direction::West);
    assert_eq!(fa.actor, shooter);
    assert_eq!(fa.facing, Direction::West);

    // Downed requests carry BOTH actor + target entities.
    let st = StabilizeDownedRequested::new(shooter, target);
    assert_eq!(st.actor, shooter);
    assert_eq!(st.target, target);
    let ex = ExecuteDownedRequested::new(shooter, target);
    assert_eq!(ex.actor, shooter);
    assert_eq!(ex.target, target);

    // MoveRequested carries the actor Entity + an OWNED CellLevel destination (no
    // borrow → no lifetime parameter; that this compiles as `MoveRequested` without
    // `<'_>` is the no-lifetime proof).
    let dest = CellLevel::new(Cell::new(7, 3), Level::new(0));
    let mv = MoveRequested::new(shooter, dest);
    assert_eq!(mv.actor, shooter, "MoveRequested carries the actor Entity");
    assert_eq!(
        mv.dest, dest,
        "MoveRequested carries the OWNED CellLevel dest"
    );
}

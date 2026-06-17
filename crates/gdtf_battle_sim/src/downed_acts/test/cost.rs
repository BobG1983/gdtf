//! AC7 — the verbs READ the flat TU cost from tuning (the leaf is genuinely
//! consulted), without running the TU economy (E4). The shipped-RON parse pin lives
//! in `tuning::tests`.

use super::support::{
    CombatTuning, ExecuteTu, LifeState, StabilizeTu, Stabilized, execute_downed, execute_pass,
    stabilize_downed, stabilize_pass,
};

/// On success each act returns the cost it READ from tuning — equal to that
/// tuning leaf (a RELATION to the value, never a pinned magnitude). This proves
/// the leaf is non-vacuously read.
#[test]
fn acts_return_the_tu_cost_read_from_tuning() {
    // A non-default tuning so the returned cost provably came FROM tuning, not a
    // hardcoded constant (the values themselves stay arbitrary, not pinned).
    let tuning = CombatTuning {
        stabilize_tu: StabilizeTu::new(9),
        execute_tu: ExecuteTu::new(13),
        ..CombatTuning::default()
    };

    let (a, t) = stabilize_pass();
    let mut flag = Stabilized::new(false);
    let stab_cost = stabilize_downed(&a, &t, &mut flag, &tuning);
    assert_eq!(
        stab_cost,
        Some(tuning.stabilize_tu),
        "stabilize_downed must return the stabilize_tu READ from tuning",
    );

    let (ea, et) = execute_pass();
    let mut life = LifeState::Downed;
    let exec_cost = execute_downed(&ea, &et, &mut life, &tuning);
    assert_eq!(
        exec_cost,
        Some(tuning.execute_tu),
        "execute_downed must return the execute_tu READ from tuning",
    );
}

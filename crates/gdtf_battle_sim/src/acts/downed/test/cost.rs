use super::support::{
    CombatTuning, ExecuteTu, LifeState, StabilizeTu, execute_downed, execute_pass, run_stabilize,
    stabilize_pass,
};

#[test]
fn acts_return_the_tu_cost_read_from_tuning() {
    let tuning = CombatTuning {
        stabilize_tu: StabilizeTu::new(9),
        execute_tu: ExecuteTu::new(13),
        ..CombatTuning::default()
    };

    let (a, t) = stabilize_pass();
    let (stab_cost, _) = run_stabilize(&a, &t, &tuning);
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

use super::{
    super::{OccupancyGrid, pathable_neighbors},
    support::*,
};
use crate::{ganger::Tu, injuries::MovementCostFactor, metric::CellLevel};

fn orthogonal_step_cost(target: CellLevel, factor: MovementCostFactor) -> Option<Tu> {
    let grid = OccupancyGrid::new(); 
    let floor_costs = default_floor_costs();
    let origin = key(target.x - 1, target.y, 0);
    pathable_neighbors(origin, &grid, &floor_costs, factor)
        .find(|(cell, _)| *cell == target)
        .map(|(_, cost)| cost)
}

#[test]
fn identity_factor_leaves_step_cost_at_base_terrain_cost() {
    let target = key(5, 5, 0);
    let base = orthogonal_step_cost(target, MovementCostFactor::IDENTITY);
    assert!(base.is_some(), "the open neighbour is reachable");
    assert_eq!(
        base,
        orthogonal_step_cost(target, MovementCostFactor::new(1.0)),
        "IDENTITY (1.0) leaves the step at the base terrain cost"
    );
}

#[test]
fn hampered_factor_scales_step_cost_by_ceil_of_product() {
    let target = key(5, 5, 0);
    let base = orthogonal_step_cost(target, MovementCostFactor::IDENTITY);
    assert!(base.is_some(), "the open neighbour must be reachable");
    let Some(base) = base else { return };
    let base = u32::from(*base);
    for mult in [1.5_f32, 2.0, 2.5, 3.0] {
        let scaled = orthogonal_step_cost(target, MovementCostFactor::new(mult));
        assert!(
            scaled.is_some(),
            "the open neighbour must still be reachable under a factor"
        );
        let Some(scaled) = scaled else { return };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "base is a small u8 floor cost and mult <= 3.0, so the ceil of the product \
                      fits a u8 and is non-negative — this is the test's expected reference \
                      value, computed the SAME way as the production scale_by_factor"
        )]
        let expected = (base as f32 * mult).ceil() as u32;
        assert_eq!(
            u32::from(*scaled),
            expected,
            "a factor of {mult} must scale the step to ceil(base × factor)"
        );
        assert!(
            u32::from(*scaled) >= base,
            "a factor >= 1.0 must never drop the step below the base terrain cost"
        );
    }
}

#[test]
fn step_cost_scaling_is_deterministic() {
    let target = key(5, 5, 0);
    let factor = MovementCostFactor::new(1.5);
    let first = orthogonal_step_cost(target, factor);
    let second = orthogonal_step_cost(target, factor);
    assert_eq!(
        first, second,
        "the same factor + terrain must produce the same scaled cost every time"
    );
}

use gdtf_ui::focus_nav::NavDirection;

use super::super::focus_step::direction;
use crate::dev::net_qa::wire::key::FocusStepNet;

#[test]
fn every_step_names_the_compass_edge_the_arrow_keys_and_the_panel_nav_use() {
    for (step, expected, arrow) in [
        (FocusStepNet::Next, NavDirection::DOWN, "arrow-down"),
        (FocusStepNet::Prev, NavDirection::UP, "arrow-up"),
        (FocusStepNet::Left, NavDirection::WEST, "arrow-left"),
        (FocusStepNet::Right, NavDirection::EAST, "arrow-right"),
    ] {
        match step {
            FocusStepNet::Next | FocusStepNet::Prev | FocusStepNet::Left | FocusStepNet::Right => {}
        }
        assert_eq!(
            direction(step),
            expected,
            "`{step:?}` must step the same compass edge {arrow} steps, or a client walking a \
             panel with it lands on a different widget than the key does",
        );
    }
}

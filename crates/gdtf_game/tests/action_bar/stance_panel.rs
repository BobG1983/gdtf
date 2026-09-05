use bevy::{ecs::entity::Entity, prelude::*};
use cobalt_test_utils::press_ui_button;
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_sim::prelude::{Direction, StanceKind};
use gdtf_game::test_support::{StanceKneelingButton, StanceProneButton, StanceStandingButton};

use super::{harness::*, probes::*};

#[test]
fn stance_panel_marks_current_stance_active() {
    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Crouching,
        Direction::North,
    );
    app.update();

    assert!(
        segment_is_active::<StanceKneelingButton>(&mut app),
        "the selected ganger's current stance (kneel) segment must be the active segment",
    );
    assert!(
        !segment_is_active::<StanceStandingButton>(&mut app)
            && !segment_is_active::<StanceProneButton>(&mut app),
        "the other two stance segments must NOT be active (mutually exclusive)",
    );

    arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Prone,
        Direction::North,
    );
    app.update();
    assert!(
        segment_is_active::<StanceProneButton>(&mut app),
        "selecting a prone ganger must move the active mark to the Prone segment",
    );
    assert!(
        !segment_is_active::<StanceStandingButton>(&mut app)
            && !segment_is_active::<StanceKneelingButton>(&mut app),
        "the active stance mark must be exclusive after the re-selection",
    );
}

#[test]
fn prone_toggle_sets_stance_prone_directly() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    let ganger = arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let Some(prone) = require_button::<StanceProneButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, prone);
    app.update();

    let via_button = stances(&app);
    assert_eq!(
        via_button.len(),
        1,
        "one SetStanceRequested via the Prone toggle",
    );
    assert_eq!(via_button[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_button[0].stance,
        StanceKind::Prone,
        "the Prone toggle DIRECT-sets the prone posture (not a cycle step)",
    );

    let mut app2 = battle_running_app();
    add_probes(&mut app2);
    let _ganger2 = arm_and_select(
        &mut app2,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app2.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::SetStance(StanceKind::Prone));
    app2.update();
    let via_intent = stances(&app2);
    assert_eq!(via_intent.len(), 1, "one SetStanceRequested via the intent");
    assert_eq!(
        via_button[0], via_intent[0],
        "the Prone toggle and the direct SetStance intent must produce byte-for-byte equal \
         SetStanceRequested",
    );
}

#[test]
fn each_stance_segment_sets_its_stance() {
    for (label, marker_press, start, expected) in [
        (
            "Stand",
            StanceSegment::Standing,
            StanceKind::Prone,
            StanceKind::Standing,
        ),
        (
            "Kneel",
            StanceSegment::Kneeling,
            StanceKind::Standing,
            StanceKind::Crouching,
        ),
        (
            "Prone",
            StanceSegment::Prone,
            StanceKind::Standing,
            StanceKind::Prone,
        ),
    ] {
        let mut app = battle_running_app();
        add_probes(&mut app);
        let ganger = arm_and_select(&mut app, sbf_selector(), start, Direction::North);
        app.update();
        app.update();
        let Some(segment) = marker_press.entity(&mut app) else {
            return;
        };
        press_ui_button(&mut app, segment);
        app.update();

        let pushed = stances(&app);
        assert_eq!(
            pushed.len(),
            1,
            "{label} segment must push exactly one SetStanceRequested",
        );
        assert_eq!(pushed[0].actor, ganger, "{label}: actor = *SelectedShooter");
        assert_eq!(
            pushed[0].stance, expected,
            "the {label} segment must direct-set {expected:?}",
        );
    }
}

enum StanceSegment {
    Standing,
    Kneeling,
    Prone,
}

impl StanceSegment {
    fn entity(&self, app: &mut App) -> Option<Entity> {
        match self {
            Self::Standing => single_with::<StanceStandingButton>(app),
            Self::Kneeling => single_with::<StanceKneelingButton>(app),
            Self::Prone => single_with::<StanceProneButton>(app),
        }
    }
}

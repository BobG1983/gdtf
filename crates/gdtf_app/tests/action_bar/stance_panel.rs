//! GTW-267 stance 3-toggle sub-panel.

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{StanceKneelingButton, StanceProneButton, StanceStandingButton};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_sim::{Direction, StanceKind};
use gdtf_test_utils::press_ui_button;

use super::{harness::*, probes::*};

// =================================================================================
// GTW-267 — the Stance 3-toggle sub-panel (replaces the blind cycle).
// =================================================================================

/// GTW-267 — selecting an armed ganger marks ITS current stance toggle `ActiveButton`
/// and ONLY that one (mutually exclusive); a fresh selection in another stance moves the
/// mark. Discriminating: a panel with no sync (the old blind cycle) would mark none.
#[test]
fn stance_panel_marks_current_stance_active() {
    let mut app = battle_running_app();
    // Select a KNEELING ganger.
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

    // Selecting a PRONE ganger moves the active mark to Prone.
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

/// GTW-267 — pressing the Prone toggle DIRECT-sets the actor's stance to Prone (one
/// `SetStanceRequested` for `*SelectedShooter` with `StanceKind::Prone`, byte-for-byte
/// EQUAL to the direct `SetStance(Prone)` intent), regardless of the current stance — NOT
/// a blind cycle step.
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

    // Byte-for-byte equal to the direct SetStance(Prone) intent over the SAME seam.
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

/// GTW-277 (new — the `SegmentSelected` → `SetStance` index→kind MAPPING) — selecting EACH
/// stance segment pushes the matching `ActIntent::SetStance(kind)` (Stand → Standing, Kneel
/// → Crouching, Prone → Prone), proving the `stance_segment_intent` listener maps the
/// widget's segment INDEX to the right `StanceKind`, not just the Prone case.
#[test]
fn each_stance_segment_sets_its_stance() {
    for (label, marker_press, start, expected) in [
        // Start from a stance DIFFERENT from the target so pressing the segment is a REAL
        // active-segment change (re-pressing the already-active segment is a widget no-op,
        // the GTW-276 set_if_neq behavior).
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
        // Settle the segment tagging + the active-segment sync to the ganger's STARTING
        // stance, so pressing the target segment is a real change (emits SegmentSelected →
        // SetStance).
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

/// Picks the right stance-segment entity by its per-stance marker for
/// [`each_stance_segment_sets_its_stance`] (a small dispatch so the loop can press each).
enum StanceSegment {
    Standing,
    Kneeling,
    Prone,
}

impl StanceSegment {
    /// The single entity carrying this stance's segment marker, if exactly one exists.
    fn entity(&self, app: &mut App) -> Option<Entity> {
        match self {
            Self::Standing => single_with::<StanceStandingButton>(app),
            Self::Kneeling => single_with::<StanceKneelingButton>(app),
            Self::Prone => single_with::<StanceProneButton>(app),
        }
    }
}

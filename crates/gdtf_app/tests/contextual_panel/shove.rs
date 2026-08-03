use bevy::prelude::*;
use gdtf_app::test_support::ShoveButton;
use gdtf_battle_input::contextual::{ContextualActSystems, PendingContextualIntents, ShoveAct};
use gdtf_battle_sim::{acts::ShoveRequested, prelude::Position};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

fn add_shove_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ShoveRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ShoveRequested>.after(ContextualActSystems::Drain),
    );
}

fn shoves(app: &App) -> Vec<ShoveRequested> {
    probed::<ShoveRequested>(app)
}


#[test]
fn adjacent_alive_opposing_offers_shove() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        shove_visible(&mut app),
        "an 8-adjacent alive opposing ganger must reveal the Shove button (GTW-525)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Shove must reveal the panel root",
    );
    assert!(
        !melee_visible(&mut app),
        "a stance/facing-less actor offers NO melee, yet Shove still reveals (weaker gate)",
    );
    assert!(
        !execute_visible(&mut app),
        "an ALIVE enemy is no Execute target (Execute needs a DOWNED enemy) -> hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

#[test]
fn non_adjacent_ally_or_downed_does_not_offer_shove() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    spawn_alive_enemy(&mut app, 5, 6, 0);
    spawn_downed(&mut app, 4, 4, 1, None);
    app.update();

    assert!(
        !shove_visible(&mut app),
        "a non-adjacent enemy + an adjacent ally + an adjacent DOWNED enemy offer NO shove",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        shove_visible(&mut app),
        "moving the alive opposing enemy into 8-adjacency reveals the Shove button (discriminating)",
    );
}

#[test]
fn pressing_shove_emits_shove_requested_for_target() {
    let mut app = battle_running_app();
    add_shove_probe(&mut app);
    let shover = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    app.update();
    assert!(
        shove_visible(&mut app),
        "sanity: the Shove button is offered before the press",
    );
    let Some(shove_btn) = single_with::<ShoveButton>(&mut app) else {
        return;
    };

    press_ui_button(&mut app, shove_btn);
    app.update();

    let emitted = shoves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Shove with a target offered must emit exactly one ShoveRequested",
    );
    assert_eq!(
        emitted[0].shover, shover,
        "the shover is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried opposing neighbour",
    );
}

#[test]
fn contextual_press_drains_the_same_update_it_was_queued() {
    let mut app = battle_running_app();
    add_shove_probe(&mut app);
    let shover = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    app.update();
    assert!(
        shove_visible(&mut app),
        "sanity: the Shove button is offered before the press",
    );
    let Some(shove_btn) = single_with::<ShoveButton>(&mut app) else {
        return;
    };

    press_ui_button(&mut app, shove_btn);
    app.update();

    let emitted = shoves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the press queued this update must be drained to its *Requested THIS update",
    );
    assert_eq!(
        emitted[0].shover, shover,
        "the shover is the SelectedShooter"
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the offered opposing neighbour",
    );
    assert!(
        app.world()
            .resource::<PendingContextualIntents<ShoveAct>>()
            .is_empty(),
        "the per-act queue is EMPTIED by the same-update drain (acted on exactly once)",
    );
}

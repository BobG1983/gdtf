use bevy::prelude::*;
use gdtf_battle_input::contextual::{ContextualActSystems, PendingContextualIntents, ShoveAct};
use gdtf_battle_sim::{acts::ShoveRequested, ganger::Tu, prelude::Position};
use gdtf_game::test_support::ShoveButton;
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
        "an 8-adjacent alive opposing ganger must reveal the Shove button",
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
    let shove_btn = the_only::<ShoveButton>(
        &mut app,
        "the panel must offer exactly one Shove button to press",
    );

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

/// A shover one TU short of the cost, standing beside a legal target.
fn a_shover_one_tu_short() -> (App, Entity) {
    let mut app = battle_running_app();
    add_shove_probe(&mut app);
    let shover = spawn_actor(&mut app, 5, 5, 0);
    spawn_alive_enemy(&mut app, 6, 6, 1);

    let cost = shove_cost(&app);
    assert!(
        *cost > 0,
        "a shove must cost something or an unaffordable pool cannot exist",
    );
    set_pool(&mut app, shover, Tu::new(cost.saturating_sub(1)));
    app.update();
    (app, shover)
}

#[test]
fn a_pool_below_the_shove_cost_still_offers_the_button_greyed_out() {
    let (mut app, _shover) = a_shover_one_tu_short();

    assert!(
        shove_visible(&mut app),
        "an unaffordable shove is still OFFERED — the player sees the act and why it is barred",
    );
    assert!(
        button_greyed::<ShoveButton>(&mut app),
        "a pool below the shove cost greys the button out",
    );
    let (greyed, _idle) = greyed_and_idle_fills(&app);
    assert_eq!(
        button_fill::<ShoveButton>(&mut app),
        Some(greyed),
        "the greyed button is painted the theme's disabled fill",
    );
}

#[test]
fn neither_a_press_nor_a_slot_key_fires_a_greyed_shove() {
    let (mut app, _shover) = a_shover_one_tu_short();
    let shove_btn = the_only::<ShoveButton>(
        &mut app,
        "the panel must still offer exactly one Shove button while it is greyed out, or there is \
         nothing for this case to press",
    );

    press_ui_button(&mut app, shove_btn);
    press_digit(&mut app, KeyCode::Digit1);
    app.update();

    assert!(
        shoves(&app).is_empty(),
        "a greyed button emits nothing from a mouse press or from its slot key",
    );
}

#[test]
fn raising_the_pool_to_the_cost_re_enables_and_repaints_the_shove_button() {
    let (mut app, shover) = a_shover_one_tu_short();
    assert!(
        button_greyed::<ShoveButton>(&mut app),
        "sanity: the button is greyed before the pool is raised",
    );

    let covering = shove_cost(&app);
    set_pool(&mut app, shover, covering);
    app.update();
    app.update();

    assert!(
        shove_visible(&mut app),
        "a pool that covers the cost keeps the button on screen",
    );
    assert!(
        !button_greyed::<ShoveButton>(&mut app),
        "a pool that covers the cost drops the disabled marker",
    );
    let (greyed, idle) = greyed_and_idle_fills(&app);
    assert_ne!(
        greyed, idle,
        "the theme must paint disabled and idle differently or this case cannot discriminate",
    );
    assert_eq!(
        button_fill::<ShoveButton>(&mut app),
        Some(idle),
        "re-enabling repaints the button back to its enabled fill",
    );

    let shove_btn = the_only::<ShoveButton>(
        &mut app,
        "the panel must offer exactly one Shove button once the pool covers the cost",
    );
    press_ui_button(&mut app, shove_btn);
    app.update();
    assert_eq!(
        shoves(&app).len(),
        1,
        "the re-enabled button presses through to exactly one ShoveRequested",
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
    let shove_btn = the_only::<ShoveButton>(
        &mut app,
        "the panel must offer exactly one Shove button to press",
    );

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

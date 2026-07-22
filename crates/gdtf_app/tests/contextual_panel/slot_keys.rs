//! GTW-563 — keyboard slot-bindings for contextual acts, driven through the REAL app
//! stack.
//!
//! The UX rule: a DIGIT key activates the Nth CURRENTLY-VISIBLE contextual button
//! (1-based, `PanelSlot` order) — the binding is per-SLOT, not per-action, so which act a
//! digit fires shifts as buttons appear and disappear. These headless tests offer three
//! acts simultaneously and drive synthesized digit presses through the real dispatch path
//! (the ranking pass → the per-act `press_contextual_button_via_key::<A>` → the SAME
//! `PendingContextualIntents<A>` a click uses → the act's generic drain → the `*Requested`
//! the sim consumes).
//!
//! With the actor at (5,5) the offered acts and their fixed `PanelSlot`s are:
//! Execute (slot 0, an adjacent downed ENEMY), Stabilize (slot 1, an adjacent unstabilized
//! downed ALLY), Shove (slot 3, an adjacent alive opposing ENEMY). Ranked among the
//! visible set that is Execute→1 (Digit1), Stabilize→2 (Digit2), Shove→3 (Digit3).

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    prelude::*,
};
use gdtf_battle_input::contextual::ContextualActSystems;
use gdtf_battle_sim::acts::{ExecuteDownedRequested, ShoveRequested, StabilizeDownedRequested};
use gdtf_test_utils::{MessageProbe, drain_message_probe, probed};

use super::{actors::*, harness::*};

/// Synthesizes a real just-pressed keypress by writing a [`KeyboardInput`] message the
/// harness's `keyboard_input_system` (`PreUpdate`) turns into a `just_pressed` edge on
/// `ButtonInput<KeyCode>` for that frame's Update — a direct `ButtonInput::press` would be
/// wiped by that same system's per-frame clear, since the battle harness runs `InputPlugin`.
/// The edge is auto-cleared next frame, so each press is a clean one-shot (no `clear_keys`).
fn press_digit(app: &mut App, key_code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Character(" ".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// Adds probes for the three acts these tests exercise — each drains `.after` the GTW-571
/// contextual drain set, so it observes the digit press's same-update emission.
fn add_probes(app: &mut App) {
    app.init_resource::<MessageProbe<ExecuteDownedRequested>>();
    app.init_resource::<MessageProbe<StabilizeDownedRequested>>();
    app.init_resource::<MessageProbe<ShoveRequested>>();
    app.add_systems(
        Update,
        (
            drain_message_probe::<ExecuteDownedRequested>,
            drain_message_probe::<StabilizeDownedRequested>,
            drain_message_probe::<ShoveRequested>,
        )
            .after(ContextualActSystems::Drain),
    );
}

/// Cumulative counts collected so far.
fn executes(app: &App) -> usize {
    probed::<ExecuteDownedRequested>(app).len()
}
fn stabilizes(app: &App) -> usize {
    probed::<StabilizeDownedRequested>(app).len()
}
fn shoves(app: &App) -> usize {
    probed::<ShoveRequested>(app).len()
}

/// Spawns the actor with all three acts offered at once and settles one detection update.
/// Execute (rank 1) / Stabilize (rank 2) / Shove (rank 3) are all visible on return.
fn app_with_three_acts_offered() -> App {
    let mut app = battle_running_app();
    add_probes(&mut app);
    spawn_actor(&mut app, 5, 5, 0);
    // Execute — an 8-adjacent downed ENEMY (slot 0).
    spawn_downed(&mut app, 4, 4, 1, None);
    // Stabilize — an 8-adjacent unstabilized downed ALLY (slot 1).
    spawn_downed(&mut app, 5, 4, 0, Some(false));
    // Shove — an 8-adjacent alive opposing ENEMY (slot 3). The plain actor carries no
    // stance/facing, so no Melee is ever offered (only these three show).
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();
    assert!(
        execute_visible(&mut app) && stabilize_visible(&mut app) && shove_visible(&mut app),
        "sanity: exactly the three acts (Execute/Stabilize/Shove) are offered before the press",
    );
    app
}

/// A digit key activates the CURRENTLY-VISIBLE button of its matching rank: with three
/// acts offered, Digit2 fires the RANK-2 act (Stabilize) and nothing else — proving the
/// digit maps to a visible SLOT, driven through the real per-act key-press path.
#[test]
fn digit_activates_the_matching_visible_rank() {
    let mut app = app_with_three_acts_offered();

    // Digit2 -> the rank-2 visible button (Stabilize).
    press_digit(&mut app, KeyCode::Digit2);
    app.update();

    assert_eq!(
        stabilizes(&app),
        1,
        "Digit2 must fire exactly the rank-2 act (Stabilize) through the digit-key dispatch path",
    );
    assert_eq!(
        executes(&app),
        0,
        "Digit2 must NOT fire the rank-1 act (Execute)",
    );
    assert_eq!(
        shoves(&app),
        0,
        "Digit2 must NOT fire the rank-3 act (Shove)"
    );
}

/// The binding is per-SLOT, not per-action — the discriminating slot-not-action pin, read
/// against `digit_activates_the_matching_visible_rank`. There, with THREE acts offered
/// (Execute rank 1, Stabilize rank 2, Shove rank 3), Digit2 fired Stabilize. HERE only two
/// acts are offered — Stabilize (slot 1) and Shove (slot 3), Execute absent — so the ranks
/// are Stabilize→1 (Digit1) and Shove→2 (Digit2), and the SAME Digit2 fires a DIFFERENT act
/// (Shove) while Stabilize (now Digit1) stays silent. Same key, different visible slot ⇒
/// different act.
#[test]
fn digit_binds_to_slot_not_action_with_a_different_visible_set() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    spawn_actor(&mut app, 5, 5, 0);
    // Stabilize — an 8-adjacent unstabilized downed ALLY (slot 1 → rank 1 / Digit1).
    spawn_downed(&mut app, 5, 4, 0, Some(false));
    // Shove — an 8-adjacent alive opposing ENEMY (slot 3 → rank 2 / Digit2). NO downed enemy
    // is adjacent, so Execute is NOT offered — exactly two acts show.
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();
    assert!(
        stabilize_visible(&mut app) && shove_visible(&mut app) && !execute_visible(&mut app),
        "sanity: exactly two acts (Stabilize→rank1, Shove→rank2) are offered, Execute absent",
    );

    // The SAME Digit2 that fired Stabilize with THREE acts offered now fires Shove — the
    // binding follows the visible SLOT (rank 2 here is Shove), not a fixed action.
    press_digit(&mut app, KeyCode::Digit2);
    app.update();

    assert_eq!(
        shoves(&app),
        1,
        "with this visible set Digit2 fires the rank-2 act (Shove) — the binding is per-slot",
    );
    assert_eq!(
        stabilizes(&app),
        0,
        "Stabilize is rank 1 (Digit1) here, so Digit2 does NOT fire it",
    );
}

/// A digit beyond the currently-visible count does nothing (no crash, no dispatch to a
/// nonexistent slot): with three acts offered (ranks 1..3), Digit4 fires nothing.
#[test]
fn digit_beyond_visible_count_is_a_noop() {
    let mut app = app_with_three_acts_offered();

    // Digit4 — no visible button has rank 4.
    press_digit(&mut app, KeyCode::Digit4);
    app.update();

    assert_eq!(
        executes(&app) + stabilizes(&app) + shoves(&app),
        0,
        "a digit past the visible count (Digit4 with three acts) dispatches nothing",
    );
}

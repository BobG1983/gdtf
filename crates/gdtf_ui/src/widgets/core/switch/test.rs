//! Tests for the `Switch` widget behavior (the `switch.rs` surface).
//!
//! Runs on the shared in-crate harness (see
//! [`test_support`](crate::widgets::core::test_support)). The tests pin the
//! click-flip + `ToggleFlipped` message contract, both orientations, and the
//! GTW-277 pill geometry (the track frames the knob).

use bevy::{
    ecs::system::SystemState,
    prelude::*,
    ui::{BackgroundColor, Interaction, Node, Val},
};

use super::{SwitchColors, SwitchKnob, SwitchState, ToggleFlipped, spawn_switch};
use crate::widgets::core::{
    Orientation,
    test_support::{LOST, REMAINING, harness},
};

/// A caller-attached identity marker on a switch, used to confirm the flip message
/// maps back to the right widget.
#[derive(Component, Clone, Copy)]
struct AimToggle;

/// The knob child of the switch rooted at `switch`, if any.
fn knob_of(app: &mut App, switch: Entity) -> Option<Entity> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let Ok(children) = state.get(app.world()) else {
        return None;
    };
    children
        .get(switch)
        .ok()?
        .iter()
        .find(|&c| app.world().get::<SwitchKnob>(c).is_some())
}

/// Presses the switch (sets `Interaction::Pressed`) and runs one update so
/// `drive_switches` fires.
fn click_switch(app: &mut App, switch: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(switch) {
        *interaction = Interaction::Pressed;
    }
    app.update();
}

/// AC — a click flips the state, emits `ToggleFlipped` with the right identity, and
/// the TRACK color reflects the new state while the knob color is unchanged. The
/// switch entity is stable.
#[test]
fn switch_click_flips_emits_and_recolors_track() {
    let mut app = harness();
    let track_off = Color::srgb(0.1, 0.1, 0.1);
    let track_on = Color::srgb(0.3, 0.7, 0.3);
    let knob_color = Color::WHITE;
    let switch = {
        let mut commands = app.world_mut().commands();
        spawn_switch(
            &mut commands,
            SwitchState::Off,
            SwitchColors {
                off:  track_off,
                on:   track_on,
                knob: knob_color,
            },
            Orientation::Horizontal,
            AimToggle,
        )
    };
    app.world_mut().flush();

    let maybe_knob = knob_of(&mut app, switch);
    assert!(maybe_knob.is_some(), "switch must have a knob child");
    let Some(knob) = maybe_knob else { return };
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(track_off),
        "precondition: starts in the off track color",
    );

    click_switch(&mut app, switch);

    // State flipped to On.
    assert_eq!(
        app.world().get::<SwitchState>(switch).copied(),
        Some(SwitchState::On),
        "the click must flip the stored state to On",
    );
    // The track recolored to the on color; the knob color is unchanged.
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(track_on),
        "the TRACK must take the on color after the flip",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(knob).map(|c| c.0),
        Some(knob_color),
        "the KNOB color must be unchanged by the flip",
    );
    // The flip message carries this switch's identity, and the entity is stable.
    let mut state: SystemState<MessageReader<ToggleFlipped>> = SystemState::new(app.world_mut());
    let reader_result = state.get_mut(app.world_mut());
    assert!(reader_result.is_ok(), "MessageReader params must validate");
    let Ok(mut reader) = reader_result else {
        return;
    };
    let msgs: Vec<ToggleFlipped> = reader.read().copied().collect();
    assert_eq!(msgs.len(), 1, "exactly one ToggleFlipped per click");
    assert_eq!(msgs[0].switch, switch, "the message carries the switch id");
    assert_eq!(
        msgs[0].state,
        SwitchState::On,
        "the message carries the new state"
    );
    assert!(
        app.world().get::<AimToggle>(switch).is_some(),
        "the switch entity (with its caller marker) is stable across the flip",
    );
}

/// AC — both orientations lay out: a horizontal switch is wider than tall, a vertical
/// switch is taller than wide.
#[test]
fn switch_supports_both_orientations() {
    let mut app = harness();
    let colors = SwitchColors {
        off:  LOST,
        on:   REMAINING,
        knob: Color::WHITE,
    };
    let (h, v) = {
        let mut commands = app.world_mut().commands();
        let h = spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Horizontal,
            (),
        );
        let v = spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Vertical,
            (),
        );
        (h, v)
    };
    app.world_mut().flush();

    let maybe_h = app.world().get::<Node>(h).cloned();
    let maybe_v = app.world().get::<Node>(v).cloned();
    assert!(
        maybe_h.is_some() && maybe_v.is_some(),
        "both switches have a Node"
    );
    let (Some(hn), Some(vn)) = (maybe_h, maybe_v) else {
        return;
    };
    // The two orientations swap width/height, so the horizontal track is the
    // mirror of the vertical track.
    assert_eq!(hn.width, vn.height, "horizontal width == vertical height");
    assert_eq!(hn.height, vn.width, "horizontal height == vertical width");
    assert_ne!(hn.width, hn.height, "a switch track is not square");
}

/// The `Vw` magnitude of a [`Val`], for proportion comparisons. Returns `None`
/// for any non-`Vw` unit so a unit regression fails the assert rather than
/// silently comparing across kinds.
fn vw(val: Val) -> Option<f32> {
    match val {
        Val::Vw(v) => Some(v),
        _ => None,
    }
}

/// AC (GTW pill fix) — the knob is a pip INSIDE a visible track, not a circle that
/// FILLS it: the track's SHORT dimension is strictly TALLER than the knob diameter
/// so track shows around the knob on the short axis, AND the track's LONG dimension
/// exceeds the knob so the knob has travel. All dims stay `Vw` (the orientation-swap
/// invariant). Pin-discriminating: shrinking the track short dim back to the knob
/// diameter (the original invisible-track bug) fails the first assert.
#[test]
fn switch_track_frames_the_knob() {
    let mut app = harness();
    let colors = SwitchColors {
        off:  LOST,
        on:   REMAINING,
        knob: Color::WHITE,
    };
    let switch = {
        let mut commands = app.world_mut().commands();
        spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Horizontal,
            (),
        )
    };
    app.world_mut().flush();

    let maybe_knob = knob_of(&mut app, switch);
    assert!(maybe_knob.is_some(), "switch must have a knob child");
    let Some(knob) = maybe_knob else { return };

    let track = app.world().get::<Node>(switch).cloned();
    let knob_node = app.world().get::<Node>(knob).cloned();
    let (Some(track), Some(knob_node)) = (track, knob_node) else {
        unreachable!("the track and knob both have a Node after flush");
    };

    // Horizontal: width is the LONG axis, height the SHORT axis.
    let (Some(long), Some(short)) = (vw(track.width), vw(track.height)) else {
        unreachable!("both track dims are Vw");
    };
    let (Some(knob_w), Some(knob_h)) = (vw(knob_node.width), vw(knob_node.height)) else {
        unreachable!("both knob dims are Vw");
    };
    assert!(
        short > knob_h,
        "the track SHORT dimension ({short}) must exceed the knob diameter ({knob_h}) \
         so track shows around the knob (the knob is a pip, not a fill)",
    );
    assert!(
        long > knob_w,
        "the track LONG dimension ({long}) must exceed the knob ({knob_w}) so the knob travels",
    );
}

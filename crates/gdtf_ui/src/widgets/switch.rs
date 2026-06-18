//! The [`Switch`] widget: a 2-way toggle — a knob in a rounded track.
//!
//! A switch is the AIM control in the mockup: a rounded TRACK whose color tells the
//! state (off / on) with a circular KNOB that slides to one end. Clicking ANYWHERE on
//! the widget flips the state, mutates the track color + knob position in place
//! ([[ui-mutate-not-respawn]]), and emits a generic [`ToggleFlipped`] message
//! carrying the switch's IDENTITY (its [`Entity`]) so a downstream listener maps it to
//! an action. `gdtf_ui` DEFINES the message; it never knows about game acts.
//!
//! Both [`Orientation::Horizontal`] and [`Orientation::Vertical`] are supported: the
//! knob slides along the chosen axis. Off pins the knob to the start, on to the end.

use bevy::{
    prelude::*,
    ui::{
        AlignItems, BackgroundColor, BorderRadius, Interaction, JustifyContent, Node, UiRect, Val,
        widget::Button,
    },
};

use super::orientation::Orientation;

/// The on/off state of a [`Switch`].
///
/// A named two-state vocabulary rather than a bare `bool`: the message and the stored
/// state read `SwitchState::On` at the call site, not an opaque `true`. UI-level
/// plumbing, not a game-domain value.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SwitchState {
    /// The switch is OFF — track in the off color, knob at the start.
    #[default]
    Off,
    /// The switch is ON — track in the on color, knob at the end.
    On,
}

impl SwitchState {
    /// The state this switch becomes when flipped.
    #[must_use]
    pub const fn flipped(self) -> Self {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Off,
        }
    }

    /// Whether this state is ON.
    #[must_use]
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

/// The off-color / on-color pair a [`Switch`]'s TRACK shows, plus the knob color.
///
/// Pure UI plumbing ([`bevy::Color`]s): off/on are applied to the TRACK (not the
/// knob, per the contract), and the knob keeps its own constant color across the flip.
/// Carried as a [`Component`] on the switch root so [`drive_switches`] can re-derive
/// the track color from the new state without the caller re-passing colors.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct SwitchColors {
    /// Track color while OFF.
    pub off:  Color,
    /// Track color while ON.
    pub on:   Color,
    /// The knob fill color (constant across the flip).
    pub knob: Color,
}

impl SwitchColors {
    /// The track color for a given [`SwitchState`].
    #[must_use]
    pub const fn track(&self, state: SwitchState) -> Color {
        match state {
            SwitchState::Off => self.off,
            SwitchState::On => self.on,
        }
    }
}

/// Marker on the TRACK root (the clickable [`Button`]) of a [`Switch`].
///
/// The root is the rounded track; the knob is its single child marked [`SwitchKnob`].
/// The caller attaches its own identity marker alongside this so the downstream
/// listener of [`ToggleFlipped`] can map the flipped switch to an action.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Switch;

/// Marker on the KNOB child of a [`Switch`].
///
/// Its color is constant; only its parent's [`JustifyContent`](bevy::ui::JustifyContent)
/// (which end of the track it sits at) changes on a flip.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SwitchKnob;

/// The orientation of a [`Switch`], stored so [`drive_switches`] re-justifies the
/// knob along the right axis on a flip.
///
/// A [`Component`] newtype over [`Orientation`] (not a bare enum field on another
/// component) so the switch root carries its own axis.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SwitchOrientation(Orientation);

/// A buffered Bevy **message** emitted when a [`Switch`] is flipped (bevy-traps
/// rule 4: buffered events are messages in 0.18).
///
/// Carries the flipped switch's [`Entity`] — its IDENTITY. `gdtf_ui` cannot know what
/// the toggle MEANS (an act, a setting), so it reports only *which* switch changed and
/// *to what*; a downstream listener reads the switch's own caller-attached marker off
/// that entity and maps it to an action. This keeps the widget generic (it defines the
/// message, never the act).
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ToggleFlipped {
    /// The switch entity that flipped (carries the caller's identity marker).
    pub switch: Entity,
    /// The state the switch flipped TO.
    pub state:  SwitchState,
}

/// Spawns a [`Switch`] (a [`Switch`] track root with one [`SwitchKnob`] child) and
/// returns the TRACK [`Entity`].
///
/// `state` is the initial on/off state; `colors` are the track off/on + knob colors;
/// `orientation` is the slide axis ([`Orientation::Horizontal`] /
/// [`Orientation::Vertical`]); `marker` is any [`Bundle`] the caller wants on the
/// track root — typically its own identity marker so the [`ToggleFlipped`] listener
/// can map it to an action.
///
/// The track is a [`Button`] so `bevy_ui`'s built-in `ui_focus_system` drives its
/// [`Interaction`](bevy::ui::Interaction) from the mouse (bevy-traps) — clicking
/// anywhere on the track is a click on the whole widget. The knob is spawned once;
/// [`drive_switches`] only re-justifies + re-colors on a flip ([[ui-mutate-not-respawn]]).
pub fn spawn_switch(
    commands: &mut Commands,
    state: SwitchState,
    colors: SwitchColors,
    orientation: Orientation,
    marker: impl Bundle,
) -> Entity {
    let (width, height) = match orientation {
        Orientation::Horizontal => (Val::Vw(TRACK_LONG_VW), Val::Vw(TRACK_SHORT_VW)),
        Orientation::Vertical => (Val::Vw(TRACK_SHORT_VW), Val::Vw(TRACK_LONG_VW)),
    };
    commands
        .spawn((
            Switch,
            SwitchOrientation(orientation),
            state,
            colors,
            Button,
            Node {
                width,
                height,
                flex_direction: orientation.flex_direction(),
                padding: UiRect::all(Val::Vw(TRACK_PAD_VW)),
                align_items: AlignItems::Center,
                justify_content: knob_justify(state),
                border_radius: BorderRadius::all(Val::Percent(50.0)),
                ..default()
            },
            BackgroundColor(colors.track(state)),
            marker,
        ))
        .with_children(|track| {
            track.spawn((
                SwitchKnob,
                Node {
                    width: Val::Vw(KNOB_DIAMETER_VW),
                    height: Val::Vw(KNOB_DIAMETER_VW),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(colors.knob),
            ));
        })
        .id()
}

/// Read-write [`Query`] data for one clicked [`Switch`]: its [`Entity`], its
/// [`Interaction`](bevy::ui::Interaction), its [`SwitchState`], its [`SwitchColors`],
/// its [`SwitchOrientation`], and its [`Node`] (for the knob justify).
///
/// Named to keep [`drive_switches`]'s signature legible (clippy `type_complexity`).
type SwitchData = (
    Entity,
    &'static Interaction,
    &'static mut SwitchState,
    &'static SwitchColors,
    &'static SwitchOrientation,
    &'static mut Node,
);

/// Flips every [`Switch`] whose [`Interaction`](bevy::ui::Interaction) `Changed` to
/// [`Pressed`](bevy::ui::Interaction::Pressed) this frame, MUTATING the stored state,
/// the track color, and the knob justify in place, then emits a [`ToggleFlipped`]
/// message.
///
/// `Changed<Interaction>` + the explicit `== Pressed` test means one flip per click
/// (the press edge), not one per frame held. The flip is mutate-in-place: the stored
/// [`SwitchState`] is overwritten, the track [`BackgroundColor`](bevy::ui::BackgroundColor)
/// is re-derived, and the knob is re-justified — no despawn/respawn
/// ([[ui-mutate-not-respawn]]).
///
/// Param-only — no `&mut World` (bevy-traps rule 7). Registered by
/// [`UiPlugin`](crate::UiPlugin) in [`Update`]; emits via
/// [`MessageWriter`](bevy::prelude::MessageWriter) (bevy-traps rule 4).
pub fn drive_switches(
    mut switches: Query<SwitchData, (Changed<Interaction>, With<Switch>)>,
    mut backgrounds: Query<&mut BackgroundColor, With<Switch>>,
    mut flipped: MessageWriter<ToggleFlipped>,
) {
    for (entity, interaction, mut state, colors, orientation, mut node) in &mut switches {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let next = state.flipped();
        *state = next;
        // Re-affirm the slide axis (it never changes after spawn, but a flip is the
        // natural place to keep the knob justify and its axis consistent) and slide
        // the knob to the new end.
        node.flex_direction = orientation.flex_direction();
        node.justify_content = knob_justify(next);
        if let Ok(mut background) = backgrounds.get_mut(entity) {
            background.0 = colors.track(next);
        }
        flipped.write(ToggleFlipped {
            switch: entity,
            state:  next,
        });
    }
}

/// Which end of the track the knob sits at for a state.
///
/// OFF pins the knob to the START of the container's main axis, ON to the END. The
/// main axis is set by [`Orientation::flex_direction`] on the track node, so this one
/// [`JustifyContent`](bevy::ui::JustifyContent) mapping slides the knob correctly for
/// both orientations.
const fn knob_justify(state: SwitchState) -> JustifyContent {
    match state {
        SwitchState::Off => JustifyContent::FlexStart,
        SwitchState::On => JustifyContent::FlexEnd,
    }
}

/// The long dimension of a switch track, in viewport-width units (the slide
/// length). `Vw` for BOTH track dims (not `Vh` for the short one) so the
/// orientation-swap test invariants hold: a horizontal track's width equals a
/// vertical track's height (both `Vw(TRACK_LONG_VW)`) AND width != height.
/// Calibrated 36px / 1280 * 100 at the default 1280x720 window.
const TRACK_LONG_VW: f32 = 2.8125;

/// The short dimension of a switch track, in viewport-width units. `Vw` (not
/// `Vh`) so it shares an axis with [`TRACK_LONG_VW`] — see that const.
/// Calibrated 18px / 1280 * 100.
const TRACK_SHORT_VW: f32 = 1.40625;

/// The inner padding around the knob inside the track, in viewport-width units.
/// Calibrated 2px / 1280 * 100.
const TRACK_PAD_VW: f32 = 0.15625;

/// The knob diameter, in viewport-width units (applied to both width + height;
/// the sub-pixel 16:9 skew on a ~14px knob is invisible). Calibrated
/// 14px / 1280 * 100.
const KNOB_DIAMETER_VW: f32 = 1.09375;

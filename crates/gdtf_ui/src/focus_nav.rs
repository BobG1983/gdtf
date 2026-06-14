//! Keyboard + gamepad focus navigation for the hand-rolled GDTF UI.
//!
//! This module is the first-party (bevy-only) focus-navigation layer the menu
//! work hangs on. It wires Bevy's [`bevy::input_focus`] framework — the
//! [`InputDispatchPlugin`](bevy::input_focus::InputDispatchPlugin) (focus
//! tracking via the [`InputFocus`](bevy::input_focus::InputFocus) resource) and
//! the
//! [`DirectionalNavigationPlugin`](bevy::input_focus::directional_navigation::DirectionalNavigationPlugin)
//! (a directed graph of focusable entities) — and bridges real device input
//! onto it. No ecosystem navigation crate is used; this is `bevy::input_focus`
//! only, pinned to 0.18.1.
//!
//! ## The pipeline (explicitly ordered — see [`FocusNavSystems`])
//!
//! 1. **Bridge** ([`FocusNavSystems::Bridge`]): the device-reading systems
//!    [`bridge_keyboard_navigation`] and [`bridge_gamepad_navigation`] translate
//!    `ArrowUp`/`ArrowDown` + `W`/`S` + gamepad D-pad up/down into a
//!    [`NavigateRequest`] message, and `Enter` / gamepad South into a
//!    [`FocusActivated`] message. They never call `navigate` directly, so the
//!    intent is a synthesizable, testable [`Message`] rather than a hidden
//!    side effect.
//! 2. **Apply** ([`FocusNavSystems::Apply`]): [`apply_navigation`] drains the
//!    [`NavigateRequest`] messages and calls
//!    [`DirectionalNavigation::navigate`](bevy::input_focus::directional_navigation::DirectionalNavigation::navigate),
//!    which moves the [`InputFocus`](bevy::input_focus::InputFocus) resource to
//!    the neighboring focusable entity.
//!
//! `Bridge` is ordered strictly `.before()` `Apply` ([`FocusNavPlugin::build`]),
//! so a navigate intent raised this frame is consumed the same frame — without
//! the ordering the two could run in either order and drop a frame (bevy-traps
//! rule 3).
//!
//! ## Activation mechanism (decision)
//!
//! Activation does **not** pass a bare entity tuple. The focused entity is
//! activated by emitting the named [`FocusActivated`] message (a newtype over
//! `Entity`). The consumer that turns activation into a menu action lands in
//! GTW-122, which reads [`FocusActivated`] with a `MessageReader`. Keeping
//! activation a typed [`Message`] (bevy-traps rule 4) is what lets the bridge
//! and the action layer be wired and tested independently.
//!
//! ## Headless behavior
//!
//! The bridge systems take the input resources as `Option<Res<…>>` /
//! fallible queries, so under `MinimalPlugins` (no `InputPlugin`) they are inert
//! rather than panicking. [`apply_navigation`] needs only the
//! [`DirectionalNavigation`](bevy::input_focus::directional_navigation::DirectionalNavigation)
//! system param (two resources both initialized by the plugins), so it runs
//! headless and is what the unit test drives directly with a synthesized
//! [`NavigateRequest`].

use bevy::{
    input::gamepad::{Gamepad, GamepadButton},
    input_focus::{
        InputDispatchPlugin, InputFocus,
        directional_navigation::{DirectionalNavigation, DirectionalNavigationPlugin},
    },
    math::CompassOctant,
    prelude::*,
};

/// A focus-navigation direction.
///
/// A named newtype over [`CompassOctant`] so a navigation direction is never a
/// bare framework enum in a [`Message`] payload (no-bare-types rule). Only the
/// vertical octants are produced by this module today
/// ([`NavDirection::UP`] / [`NavDirection::DOWN`]); the type can carry any
/// octant when horizontal navigation is added.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavDirection(pub CompassOctant);

impl NavDirection {
    /// Move focus upward — [`CompassOctant::North`].
    pub const UP: Self = Self(CompassOctant::North);
    /// Move focus downward — [`CompassOctant::South`].
    pub const DOWN: Self = Self(CompassOctant::South);
}

/// A request to move input focus one step in a [`NavDirection`].
///
/// Emitted by the bridge systems from device input and consumed by
/// [`apply_navigation`]. It is a [`Message`] (the buffered-event API in Bevy
/// 0.18; bevy-traps rule 4) so the navigate intent can be written by a test
/// without any real device, and read deterministically the same frame it is
/// raised.
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavigateRequest(pub NavDirection);

/// The focused entity has been activated (the player pressed `Enter` / gamepad
/// South while it held focus).
///
/// This is the activation seam: rather than calling into a menu action inline,
/// the bridge emits this named [`Message`] newtype over the activated `Entity`.
/// GTW-122 adds the [`MessageReader<FocusActivated>`](MessageReader) that turns
/// an activation into a concrete menu effect, so the navigation layer and the
/// action layer stay decoupled and independently testable (bevy-traps rule 4).
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusActivated(pub Entity);

/// Explicit system-ordering sets for the focus-navigation pipeline.
///
/// [`Bridge`](FocusNavSystems::Bridge) (device → intent messages) is ordered
/// strictly before [`Apply`](FocusNavSystems::Apply) (intent → focus move) so a
/// navigate intent raised this frame is applied the same frame. Without an
/// explicit order the two could run either way round and lose a frame
/// (bevy-traps rule 3).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusNavSystems {
    /// Device-reading systems that translate input into [`NavigateRequest`] /
    /// [`FocusActivated`] messages.
    Bridge,
    /// The system that drains [`NavigateRequest`] and moves the focus.
    Apply,
}

/// Sets input focus to `entity`.
///
/// The `set_initial_focus(commands, entity)` helper a scene uses to declare the
/// first focused element when it builds its menu. It overwrites the
/// [`InputFocus`](bevy::input_focus::InputFocus) resource via a queued command,
/// so it composes with the normal `Commands` flow of a UI-spawning system and
/// applies at the next command-flush. Passing it the entity directly keeps the
/// caller from having to reach for the resource itself.
pub fn set_initial_focus(commands: &mut Commands, entity: Entity) {
    commands.insert_resource(InputFocus::from_entity(entity));
}

/// The focus-navigation sub-plugin, added by
/// [`UiPlugin`](crate::UiPlugin).
///
/// Installs Bevy's focus framework
/// ([`InputDispatchPlugin`](bevy::input_focus::InputDispatchPlugin) +
/// [`DirectionalNavigationPlugin`](bevy::input_focus::directional_navigation::DirectionalNavigationPlugin)),
/// registers the [`NavigateRequest`] / [`FocusActivated`] messages, and adds the
/// bridge + apply systems to [`Update`] with the
/// [`Bridge`](FocusNavSystems::Bridge)-before-[`Apply`](FocusNavSystems::Apply)
/// ordering.
pub struct FocusNavPlugin;

impl Plugin for FocusNavPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((InputDispatchPlugin, DirectionalNavigationPlugin))
            .add_message::<NavigateRequest>()
            .add_message::<FocusActivated>()
            .configure_sets(
                Update,
                FocusNavSystems::Bridge.before(FocusNavSystems::Apply),
            )
            .add_systems(
                Update,
                (
                    (bridge_keyboard_navigation, bridge_gamepad_navigation)
                        .in_set(FocusNavSystems::Bridge),
                    apply_navigation.in_set(FocusNavSystems::Apply),
                ),
            );
    }
}

/// Translates keyboard input into navigation / activation messages.
///
/// `ArrowUp`/`W` and `ArrowDown`/`S` raise a [`NavigateRequest`]; `Enter`
/// activates the currently focused entity by raising a [`FocusActivated`]. The
/// `ButtonInput<KeyCode>` resource is taken as `Option<Res<…>>` so the system is
/// inert under `MinimalPlugins` (no `InputPlugin`) instead of panicking.
pub fn bridge_keyboard_navigation(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    focus: Res<InputFocus>,
    mut navigate: MessageWriter<NavigateRequest>,
    mut activate: MessageWriter<FocusActivated>,
) {
    let Some(keys) = keys else {
        return;
    };

    if keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        navigate.write(NavigateRequest(NavDirection::UP));
    }
    if keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        navigate.write(NavigateRequest(NavDirection::DOWN));
    }
    if keys.just_pressed(KeyCode::Enter)
        && let Some(focused) = focus.get()
    {
        activate.write(FocusActivated(focused));
    }
}

/// Translates gamepad input into navigation / activation messages.
///
/// Gamepad D-pad up/down raise a [`NavigateRequest`]; the South face button
/// activates the focused entity via [`FocusActivated`]. Iterating the
/// [`Gamepad`] components is a normal query (empty under `MinimalPlugins`), so
/// the system is inert headless.
pub fn bridge_gamepad_navigation(
    gamepads: Query<&Gamepad>,
    focus: Res<InputFocus>,
    mut navigate: MessageWriter<NavigateRequest>,
    mut activate: MessageWriter<FocusActivated>,
) {
    for gamepad in &gamepads {
        if gamepad.just_pressed(GamepadButton::DPadUp) {
            navigate.write(NavigateRequest(NavDirection::UP));
        }
        if gamepad.just_pressed(GamepadButton::DPadDown) {
            navigate.write(NavigateRequest(NavDirection::DOWN));
        }
        if gamepad.just_pressed(GamepadButton::South)
            && let Some(focused) = focus.get()
        {
            activate.write(FocusActivated(focused));
        }
    }
}

/// Drains [`NavigateRequest`] messages and moves input focus.
///
/// For each pending request it calls
/// [`DirectionalNavigation::navigate`](bevy::input_focus::directional_navigation::DirectionalNavigation::navigate),
/// which updates the [`InputFocus`](bevy::input_focus::InputFocus) resource to
/// the neighbor in that direction. A request with no neighbor (or no current
/// focus) is a no-op — `navigate` returns an error that is intentionally
/// dropped, since "can't go further that way" is normal navigation, not a
/// failure to surface.
pub fn apply_navigation(
    mut requests: MessageReader<NavigateRequest>,
    mut navigation: DirectionalNavigation,
) {
    for request in requests.read() {
        // A missing neighbor / no-focus is expected at the edges of the menu;
        // `navigate` only mutates `InputFocus` on success, so dropping the
        // `Err` leaves focus where it was.
        let _ = navigation.navigate(*request.0);
    }
}

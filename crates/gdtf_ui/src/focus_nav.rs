//! Keyboard + gamepad focus navigation for the hand-rolled GDTF UI.
//!
//! This module is the first-party (bevy-only) focus-navigation layer the menu
//! work hangs on. It wires Bevy's [`bevy::input_focus`] framework. Focus
//! tracking via the [`InputFocus`](bevy::input_focus::InputFocus) resource — and
//! the `InputDispatchPlugin` that owns it — now ships INSIDE `DefaultPlugins` (as
//! of Bevy 0.19, behind the default-on `bevy_input_focus` feature), so this
//! module only adds the
//! [`DirectionalNavigationPlugin`](bevy::input_focus::directional_navigation::DirectionalNavigationPlugin)
//! (a directed graph of focusable entities; NOT in defaults) and bridges real
//! device input onto it. No ecosystem navigation crate is used; this is
//! `bevy::input_focus` only, pinned to 0.19.
//!
//! ## Scope — the game surfaces, not the content editor
//!
//! This framework serves the hand-rolled `bevy_ui` GDTF surfaces (the menu and
//! the battlescape HUD panels), which have no built-in focus traversal of their
//! own. The `gdtf_content_editor` deliberately does **not** use it and needs no
//! bespoke focus layer: the editor's UI is `bevy_egui`, and egui's standard
//! widgets — `ui.button`, `ui.text_edit_singleline` / `ui.text_edit_multiline`,
//! `ui.selectable_value`, and `ui.add` — are all egui-focusable and
//! Tab-navigable for free. Wiring the editor into this module would only
//! duplicate traversal egui already provides, so the content-editor
//! focus-navigation question is closed with zero new code in the editor crate
//! (the egui-for-dev-surfaces decision is ADR 0003, clause 3).
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
        InputFocus,
        directional_navigation::{DirectionalNavigation, DirectionalNavigationPlugin},
    },
    math::CompassOctant,
    prelude::*,
};

/// A focus-navigation direction.
///
/// A named newtype over [`CompassOctant`] so a navigation direction is never a
/// bare framework enum in a [`Message`] payload (no-bare-types rule). It defines
/// the four cardinal directions as named constants: the vertical
/// [`NavDirection::UP`] / [`NavDirection::DOWN`] and the horizontal
/// [`NavDirection::WEST`] / [`NavDirection::EAST`]. Only the vertical pair is
/// produced by the bridge systems today; the horizontal pair exists for the
/// panel-navigation consumers, but is not yet bound to any device input — which
/// key raises a horizontal navigate is a separate, pending-design ticket.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavDirection(CompassOctant);

impl NavDirection {
    /// Move focus upward — [`CompassOctant::North`].
    pub const UP: Self = Self(CompassOctant::North);
    /// Move focus downward — [`CompassOctant::South`].
    pub const DOWN: Self = Self(CompassOctant::South);
    /// Move focus to the west (leftward) — [`CompassOctant::West`].
    pub const WEST: Self = Self(CompassOctant::West);
    /// Move focus to the east (rightward) — [`CompassOctant::East`].
    pub const EAST: Self = Self(CompassOctant::East);

    /// Wrap a [`CompassOctant`] as a focus-navigation direction.
    #[must_use]
    pub const fn new(octant: CompassOctant) -> Self {
        Self(octant)
    }
}

/// A request to move input focus one step in a [`NavDirection`].
///
/// Emitted by the bridge systems from device input and consumed by
/// [`apply_navigation`]. It is a [`Message`] (the buffered-event API in Bevy
/// 0.18; bevy-traps rule 4) so the navigate intent can be written by a test
/// without any real device, and read deterministically the same frame it is
/// raised.
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavigateRequest(NavDirection);

impl NavigateRequest {
    /// Wrap a [`NavDirection`] as a focus-navigation request message.
    #[must_use]
    pub const fn new(direction: NavDirection) -> Self {
        Self(direction)
    }
}

/// The focused entity has been activated (the player pressed `Enter` / gamepad
/// South while it held focus).
///
/// This is the activation message: rather than calling into a menu action inline,
/// the bridge emits this named [`Message`] newtype over the activated `Entity`.
/// GTW-122 adds the [`MessageReader<FocusActivated>`](MessageReader) that turns
/// an activation into a concrete menu effect, so the navigation layer and the
/// action layer stay decoupled and independently testable (bevy-traps rule 4).
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusActivated(Entity);

impl FocusActivated {
    /// Wrap the [`Entity`] that was activated while it held focus.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// The player has cancelled the current focus interaction — a "back out" /
/// dismiss signal, the cancel counterpart to [`FocusActivated`].
///
/// Where [`FocusActivated`] commits the focused entity, `FocusCancelled` backs
/// out of the current focus context without committing it; the concrete effect
/// (closing a panel, clearing a pending choice) is the reading consumer's to
/// define. It is a typed [`Message`] (bevy-traps rule 4) the framework defines
/// and registers, so a later consumer reads it with a
/// `MessageReader<FocusCancelled>`.
///
/// This module deliberately does **not** bind it to any key: which device input
/// raises a cancel is a separate, currently-blocked ticket's decision, pending a
/// user design ruling on key arbitration. No bridge system here writes it — it
/// exists as the registered message type only.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusCancelled;

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
/// Installs the navigation half of Bevy's focus framework
/// ([`DirectionalNavigationPlugin`](bevy::input_focus::directional_navigation::DirectionalNavigationPlugin)).
/// The `InputDispatchPlugin` (and the [`InputFocus`](bevy::input_focus::InputFocus)
/// resource it owns) is NOT added here: as of Bevy 0.19 it ships in
/// `DefaultPlugins`, and adding it again would be a double-add panic.
/// This plugin also registers the [`NavigateRequest`] / [`FocusActivated`] /
/// [`FocusCancelled`] messages, and adds the
/// bridge + apply systems to [`Update`] with the
/// [`Bridge`](FocusNavSystems::Bridge)-before-[`Apply`](FocusNavSystems::Apply)
/// ordering.
pub struct FocusNavPlugin;

impl Plugin for FocusNavPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DirectionalNavigationPlugin)
            // `InputFocus` is `init_resource`d by `InputDispatchPlugin` — which
            // now lives in `DefaultPlugins` (Bevy 0.19), NOT in this plugin.
            // Headless harnesses run `MinimalPlugins` + `UiPlugin` WITHOUT
            // `DefaultPlugins`, so they would never get `InputFocus` and the
            // focus-nav bridge (which takes `Res`/`ResMut<InputFocus>`) would
            // panic on the first `update()`. Init it here so the resource is
            // present wherever this plugin is; `init_resource` is idempotent, so
            // the real app's `InputDispatchPlugin` does not double-insert it.
            .init_resource::<InputFocus>()
            .add_message::<NavigateRequest>()
            .add_message::<FocusActivated>()
            // The cancel signal is registered so a consumer can read it, but this
            // module binds NO key to it — key arbitration is a separate ticket.
            .add_message::<FocusCancelled>()
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
        navigate.write(NavigateRequest::new(NavDirection::UP));
    }
    if keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        navigate.write(NavigateRequest::new(NavDirection::DOWN));
    }
    if keys.just_pressed(KeyCode::Enter)
        && let Some(focused) = focus.get()
    {
        activate.write(FocusActivated::new(focused));
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
            navigate.write(NavigateRequest::new(NavDirection::UP));
        }
        if gamepad.just_pressed(GamepadButton::DPadDown) {
            navigate.write(NavigateRequest::new(NavDirection::DOWN));
        }
        if gamepad.just_pressed(GamepadButton::South)
            && let Some(focused) = focus.get()
        {
            activate.write(FocusActivated::new(focused));
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
        let _ = navigation.navigate(***request);
    }
}

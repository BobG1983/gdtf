//! Directional focus navigation from keyboard and gamepad.

use bevy::{
    input::gamepad::{Gamepad, GamepadButton},
    input_focus::{
        InputFocus,
        directional_navigation::{DirectionalNavigation, DirectionalNavigationPlugin},
    },
    math::CompassOctant,
    prelude::*,
};

/// Direction for a focus move.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavDirection(CompassOctant);

impl NavDirection {
    /// Up / north.
    pub const UP: Self = Self(CompassOctant::North);
    /// Down / south.
    pub const DOWN: Self = Self(CompassOctant::South);
    /// Left / west.
    pub const WEST: Self = Self(CompassOctant::West);
    /// Right / east.
    pub const EAST: Self = Self(CompassOctant::East);

    /// Wrap a compass octant.
    #[must_use]
    pub const fn new(octant: CompassOctant) -> Self {
        Self(octant)
    }
}

/// Request to move focus in a direction.
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavigateRequest(NavDirection);

impl NavigateRequest {
    /// Build a navigate request.
    #[must_use]
    pub const fn new(direction: NavDirection) -> Self {
        Self(direction)
    }
}

/// The focused entity was activated (e.g. Enter / South).
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusActivated(Entity);

impl FocusActivated {
    /// Build from the activated entity.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Focus navigation was cancelled.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusCancelled;

/// System sets for focus navigation ordering.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusNavSystems {
    /// Input bridging into navigate/activate messages.
    Bridge,
    /// Apply navigate requests to directional navigation.
    Apply,
}

/// Set initial keyboard/gamepad focus to `entity`.
pub fn set_initial_focus(commands: &mut Commands, entity: Entity) {
    commands.insert_resource(InputFocus::from_entity(entity));
}

/// Plugin wiring directional navigation and input bridges.
pub struct FocusNavPlugin;

impl Plugin for FocusNavPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DirectionalNavigationPlugin)
            .init_resource::<InputFocus>()
            .add_message::<NavigateRequest>()
            .add_message::<FocusActivated>()
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

/// Map arrow/WASD keys to navigate and Enter to activate.
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

/// Map d-pad and South button to navigate/activate.
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

/// Apply queued navigate requests to Bevy directional navigation.
pub fn apply_navigation(
    mut requests: MessageReader<NavigateRequest>,
    mut navigation: DirectionalNavigation,
) {
    for request in requests.read() {
        let _ = navigation.navigate(***request);
    }
}

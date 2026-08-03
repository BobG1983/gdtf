use bevy::{
    input::gamepad::{Gamepad, GamepadButton},
    input_focus::{
        InputFocus,
        directional_navigation::{DirectionalNavigation, DirectionalNavigationPlugin},
    },
    math::CompassOctant,
    prelude::*,
};

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavDirection(CompassOctant);

impl NavDirection {
        pub const UP: Self = Self(CompassOctant::North);
        pub const DOWN: Self = Self(CompassOctant::South);
        pub const WEST: Self = Self(CompassOctant::West);
        pub const EAST: Self = Self(CompassOctant::East);

        #[must_use]
    pub const fn new(octant: CompassOctant) -> Self {
        Self(octant)
    }
}

#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavigateRequest(NavDirection);

impl NavigateRequest {
        #[must_use]
    pub const fn new(direction: NavDirection) -> Self {
        Self(direction)
    }
}

#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusActivated(Entity);

impl FocusActivated {
        #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusCancelled;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusNavSystems {
            Bridge,
        Apply,
}

pub fn set_initial_focus(commands: &mut Commands, entity: Entity) {
    commands.insert_resource(InputFocus::from_entity(entity));
}

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

pub fn apply_navigation(
    mut requests: MessageReader<NavigateRequest>,
    mut navigation: DirectionalNavigation,
) {
    for request in requests.read() {
        let _ = navigation.navigate(***request);
    }
}

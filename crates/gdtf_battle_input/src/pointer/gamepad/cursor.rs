use bevy::prelude::*;

#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursor(Vec2);

impl GamepadCursor {
        #[must_use]
    pub const fn new(pos: Vec2) -> Self {
        Self(pos)
    }

                        pub const DEFAULT_CENTRE: Vec2 = Vec2::new(640.0, 360.0);
}

impl Default for GamepadCursor {
        fn default() -> Self {
        Self(Self::DEFAULT_CENTRE)
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePointer {
            #[default]
    Mouse,
            Gamepad,
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorSpeed(f32);

impl CursorSpeed {
        #[must_use]
    pub const fn new(px_per_second: f32) -> Self {
        Self(px_per_second)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorStickDeadzone(f32);

impl CursorStickDeadzone {
        #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

pub const CURSOR_SPEED: CursorSpeed = CursorSpeed::new(900.0);

pub const CURSOR_STICK_DEADZONE: CursorStickDeadzone = CursorStickDeadzone::new(0.15);

#[must_use]
pub fn move_cursor(pos: Vec2, stick: Vec2, speed: CursorSpeed, dt: f32, window: Vec2) -> Vec2 {
    let delta = Vec2::new(stick.x, -stick.y) * *speed * dt;
    let moved = pos + delta;
    moved.clamp(Vec2::ZERO, window)
}

//! Mutable access to the drawn components playback writes.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::super::drawn::{DrawnLife, DrawnMagazine, DrawnPose, DrawnPosition, DrawnVitals};

/// Mutable access to all drawn components the cursor writes.
#[derive(SystemParam)]
pub struct DrawnWriters<'w, 's> {
    pub(in crate::playback) positions: Query<'w, 's, &'static mut DrawnPosition>,
    pub(in crate::playback) poses:     Query<'w, 's, &'static mut DrawnPose>,
    pub(in crate::playback) lives:     Query<'w, 's, &'static mut DrawnLife>,
    pub(in crate::playback) vitals:    Query<'w, 's, &'static mut DrawnVitals>,
    pub(in crate::playback) magazines: Query<'w, 's, &'static mut DrawnMagazine>,
}

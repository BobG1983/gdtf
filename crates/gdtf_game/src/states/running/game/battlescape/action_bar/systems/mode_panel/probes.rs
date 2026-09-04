//! Reads behind the mode panel: what the shooter offers and what the segments show.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::weapon::{FireMode, MeleeWeapon, ModeKind, MountedWeapon, WieldedBy, Wields};
use gdtf_ui::{Segment, SegmentIndex, set_segment_visible};

/// The fire modes the weapon a shooter fires offers.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct OfferedFireModes<'w, 's> {
    wields:  Query<'w, 's, &'static Wields>,
    weapons: Query<'w, 's, &'static FireMode, With<WieldedBy>>,
    mounted: Query<'w, 's, (), With<MountedWeapon>>,
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
}

/// Whether the weapon a shooter fires offers one fire mode.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ModeOffered(bool);

impl ModeOffered {
    /// Wrap the offered answer.
    const fn new(offered: bool) -> Self {
        Self(offered)
    }
}

impl OfferedFireModes<'_, '_> {
    /// Whether the weapon the shooter fires offers this mode.
    pub(super) fn offers(&self, shooter: Option<Entity>, kind: ModeKind) -> ModeOffered {
        ModeOffered::new(
            self.of(shooter)
                .is_some_and(|weapon| weapon.iter().any(|mode| mode.kind == kind)),
        )
    }

    /// The modes on the weapon the shooter fires, if it has one.
    fn of(&self, shooter: Option<Entity>) -> Option<&FireMode> {
        shooter
            .and_then(|shooter| self.wields.get(shooter).ok())
            .and_then(|wields| {
                wields.firing_weapon(
                    |entity| self.mounted.get(entity).is_ok(),
                    |entity| self.melee.get(entity).is_ok(),
                )
            })
            .and_then(|weapon| self.weapons.get(weapon).ok())
    }
}

/// The segment nodes of a segmented control, addressed by control and index.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ControlSegments<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    segments: Query<'w, 's, (&'static SegmentIndex, &'static mut Node), With<Segment>>,
}

impl ControlSegments<'_, '_> {
    /// Show or hide one segment of one control.
    pub(super) fn set_visible(&mut self, control: Entity, index: usize, visible: bool) {
        set_segment_visible(control, index, visible, &self.children, &mut self.segments);
    }
}

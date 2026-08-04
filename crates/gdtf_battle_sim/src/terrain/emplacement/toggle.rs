//! Occupy / vacate emplacements and spawn or despawn mounted weapons.

use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin, Query, Res, ResMut,
    Update, With,
};

use super::{EmplacementOccupant, EmplacementState, MountedWeaponEntity, MountedWeaponKey};
use crate::{
    clearance::silhouette_band,
    equipment::attachments::{AttachmentRegistry, resolve_pending_attachments},
    ganger::{Stance, StanceKind},
    occupancy::OccupancyGrid,
    occupancy_sync::{SimSystems, sync_moved_gangers},
    terrain::entity::TerrainCell,
    weapon::{MountedWeapon, WeaponRegistry, WieldedBy},
};

/// Request to occupy or vacate an emplacement.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetEmplacement {
    emplacement: Entity,
    ganger:      Entity,
    state:       EmplacementState,
}

impl SetEmplacement {
    /// Build a request for `emplacement` and `ganger` to become `state`.
    #[must_use]
    pub const fn new(emplacement: Entity, ganger: Entity, state: EmplacementState) -> Self {
        Self {
            emplacement,
            ganger,
            state,
        }
    }

    /// Occupy with this ganger.
    #[must_use]
    pub const fn occupy(emplacement: Entity, ganger: Entity) -> Self {
        Self::new(emplacement, ganger, EmplacementState::Occupied)
    }

    /// Vacate for this ganger.
    #[must_use]
    pub const fn vacate(emplacement: Entity, ganger: Entity) -> Self {
        Self::new(emplacement, ganger, EmplacementState::Vacant)
    }

    /// Emplacement entity.
    #[must_use]
    pub const fn emplacement(self) -> Entity {
        self.emplacement
    }

    /// Ganger entity.
    #[must_use]
    pub const fn ganger(self) -> Entity {
        self.ganger
    }

    /// Requested state.
    #[must_use]
    pub const fn state(self) -> EmplacementState {
        self.state
    }
}

/// Apply occupy/vacate: update state, occupant, silhouette band, and mounted weapon.
pub fn apply_emplacement_toggle(
    mut requests: MessageReader<SetEmplacement>,
    mut emplacements: Query<(
        &mut EmplacementState,
        &TerrainCell,
        Option<&MountedWeaponKey>,
        Option<&MountedWeaponEntity>,
    )>,
    stances: Query<&Stance, With<Stance>>,
    weapons: Option<Res<WeaponRegistry>>,
    attachments: Option<Res<AttachmentRegistry>>,
    mut grid: ResMut<OccupancyGrid>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok((mut state, cell, mounted_key, mounted_entity)) =
            emplacements.get_mut(request.emplacement())
        else {
            continue;
        };
        if *state == request.state() {
            continue;
        }
        *state = request.state();
        let ganger = request.ganger();
        let key = **cell;
        if *request.state().is_occupied() {
            commands
                .entity(request.emplacement())
                .insert(EmplacementOccupant::new(ganger));
            grid.set_occupant_band(key, Some(silhouette_band(StanceKind::Standing)));
            if let Some(mount) = spawn_mounted_weapon(
                &mut commands,
                ganger,
                mounted_key,
                weapons.as_deref(),
                attachments.as_deref(),
            ) {
                commands
                    .entity(request.emplacement())
                    .insert(MountedWeaponEntity::new(mount));
            }
        } else {
            commands
                .entity(request.emplacement())
                .remove::<EmplacementOccupant>();
            let stance = stances
                .get(ganger)
                .map_or(StanceKind::Standing, |stance| **stance);
            grid.set_occupant_band(key, Some(silhouette_band(stance)));
            if let Some(mount) = mounted_entity {
                commands.entity(**mount).despawn();
                commands
                    .entity(request.emplacement())
                    .remove::<MountedWeaponEntity>();
            }
        }
    }
}

fn spawn_mounted_weapon(
    commands: &mut Commands,
    occupant: Entity,
    mounted_key: Option<&MountedWeaponKey>,
    weapons: Option<&WeaponRegistry>,
    attachments: Option<&AttachmentRegistry>,
) -> Option<Entity> {
    let key = mounted_key?;
    let spec = weapons?.spec(key)?;
    let pending = resolve_pending_attachments(&spec.slots, &spec.attachments, attachments);
    let (bundle, _siblings) = spec.clone().into_bundle((**key).clone());
    Some(
        commands
            .spawn((bundle, WieldedBy::new(occupant), MountedWeapon, pending))
            .id(),
    )
}

/// Plugin that registers the emplacement toggle system.
#[derive(Debug, Default, Clone, Copy)]
pub struct EmplacementTogglePlugin;

impl Plugin for EmplacementTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetEmplacement>().add_systems(
            Update,
            apply_emplacement_toggle
                .in_set(SimSystems::Simulate)
                .after(sync_moved_gangers),
        );
    }
}

//! Occupy / vacate emplacements and spawn or despawn mounted weapons.

use bevy::{
    platform::collections::HashMap,
    prelude::{
        App, Commands, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin, Query, Res,
        Update,
    },
};

use super::{EmplacementState, EnteredFrom, MountedBy, MountedWeaponEntity, MountedWeaponKey};
use crate::{
    equipment::attachments::{AttachmentRegistry, resolve_pending_attachments},
    ganger::Position,
    metric::CellLevel,
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

/// Apply occupy/vacate: update state, occupant record, the occupant's cell, and the mount.
pub fn apply_emplacement_toggle(
    mut requests: MessageReader<SetEmplacement>,
    mut emplacements: Query<(
        &mut EmplacementState,
        &TerrainCell,
        Option<&MountedWeaponKey>,
        Option<&MountedWeaponEntity>,
    )>,
    mut positions: Query<&mut Position>,
    entered: Query<&EnteredFrom>,
    weapons: Option<Res<WeaponRegistry>>,
    attachments: Option<Res<AttachmentRegistry>>,
    mut commands: Commands,
) {
    // The entry cells recorded by occupies in this run, before `Commands` apply them.
    let mut entered_this_run: HashMap<Entity, CellLevel> = HashMap::new();
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
        let seat = **cell;
        if *request.state().is_occupied() {
            commands
                .entity(request.emplacement())
                .insert(MountedBy::new(ganger));
            if let Ok(mut position) = positions.get_mut(ganger) {
                let origin = **position;
                commands
                    .entity(request.emplacement())
                    .insert(EnteredFrom::new(origin));
                entered_this_run.insert(request.emplacement(), origin);
                *position = Position::new(seat);
            }
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
            commands.entity(request.emplacement()).remove::<MountedBy>();
            let origin = entered_this_run
                .remove(&request.emplacement())
                .or_else(|| entered.get(request.emplacement()).ok().map(|from| **from));
            if let (Some(origin), Ok(mut position)) = (origin, positions.get_mut(ganger)) {
                *position = Position::new(origin);
            }
            commands
                .entity(request.emplacement())
                .remove::<EnteredFrom>();
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
                .before(sync_moved_gangers),
        );
    }
}

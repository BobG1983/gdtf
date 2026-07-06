//! The **emplacement toggle** — the SIM mechanism that flips an [`EmplacementState`] between
//! vacant and occupied and maintains the occupant's cover band (GTW-543). Modelled on the
//! door-precedent toggle
//! ([`apply_openable_toggle`](crate::terrain::openable::apply_openable_toggle), GTW-503): a
//! buffered [`SetEmplacement`] message drives a system that mutates the emplacement's state
//! and the shared [`OccupancyGrid`].
//!
//! ## What occupy/vacate does
//!
//! - **Occupy** (an idle emplacement → manned): set [`EmplacementState::Occupied`], insert an
//!   [`EmplacementOccupant`] recording the ganger, and FORCE the occupant's published
//!   silhouette band to [`HeightBand::High`](crate::cover::HeightBand::High) at the emplacement cell
//!   (`set_occupant_band(cell, Some(High))`). The march reads the ganger occupant BEFORE cover
//!   (`shot_pipeline/march/dda.rs`), so a HIGH occupant band is literally "the occupant reads
//!   as HIGH cover".
//! - **Vacate** (a manned emplacement → idle): set [`EmplacementState::Vacant`], remove the
//!   [`EmplacementOccupant`], and RESTORE the occupant's band from its stance silhouette
//!   ([`silhouette_band`] of its [`Stance`], defaulting to standing → HIGH when the ganger has
//!   no `Stance`).
//!
//! An empty emplacement is still a cover-like structure: its
//! [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) /
//! [`BlocksVision`](crate::terrain::entity::BlocksVision) /
//! [`CoverLedger`](crate::cover::CoverLedger) entry persist
//! across occupancy, so — unlike the door toggle — this system NEVER adds/removes the blocking
//! components (occupancy does not change the emplacement's structural footprint).
//!
//! ## Mounted weapon (GTW-543 Phase 2)
//!
//! On OCCUPY the toggle resolves the emplacement's
//! [`MountedWeaponKey`](super::MountedWeaponKey) against the
//! [`WeaponRegistry`](crate::weapon::WeaponRegistry) and SPAWNS the bolted-down gun onto the
//! occupant — a [`WeaponBundle`](crate::weapon::WeaponBundle) related via
//! [`WieldedBy`](crate::weapon::WieldedBy) + the [`MountedWeapon`](crate::weapon::MountedWeapon)
//! marker — recording the spawned weapon's entity in a
//! [`MountedWeaponEntity`](super::MountedWeaponEntity) component on the emplacement. The
//! ranged-firing read PREFERS that [`MountedWeapon`](crate::weapon::MountedWeapon)-marked entity
//! (`Wields::mounted_weapon` before `Wields::ranged_weapon`), so the manning ganger fires the
//! mount, and the [`EmplacementStability`](crate::stability::EmplacementStability) seam steadies
//! its deliberately-wide cone. On VACATE the toggle DESPAWNS that recorded mounted-weapon entity
//! (the [`WieldedBy`](crate::weapon::WieldedBy) edge goes with it), reverting the occupant to its
//! own carried gun. A missing [`WeaponRegistry`](crate::weapon::WeaponRegistry) / an unresolved
//! key simply spawns no mount (fail-closed) — the band-force + occupant record still apply, so an
//! emplacement with an unresolvable gun still offers cover.
//!
//! ## Ordering
//!
//! [`apply_emplacement_toggle`] runs `.in_set(`[`SimSystems::Simulate`]`)` and
//! `.after(`[`sync_moved_gangers`]`)`: the occupancy-maintenance stance-band publish
//! ([`sync_moved_gangers`], the SAME set) writes the occupant's stance band, so the toggle
//! runs AFTER it to guarantee the occupy-frame HIGH-force is the last band write and is not
//! clobbered by the same-tick stance republish.
//!
//! `bevy-traps.md` #7: a normal system over `MessageReader` / `Query` / `ResMut` / `Commands`
//! — no `&mut World`. `bevy-traps.md` #4: a buffered `MessageReader`, drained every run.

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

/// A request to set a **weapon emplacement**'s [`EmplacementState`] — the toggle message the
/// enter/exit context acts write (GTW-543).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`, mirroring
/// [`SetOpenable`](crate::terrain::openable::SetOpenable) and the landed `*Requested` act
/// messages). Carries the emplacement [`Entity`], the TARGET [`EmplacementState`], and the
/// ganger [`Entity`] the request concerns (the occupant to seat on occupy, or the occupant to
/// unseat on vacate). `state` is an absolute set (occupy OR vacate), not a blind flip, so the
/// caller drives an explicit, idempotent outcome.
///
/// Both entities are Bevy [`Entity`] handles — framework plumbing, the only bare type the
/// no-bare-types rule permits in a message payload (they are not domain values). Build one via
/// [`occupy`](SetEmplacement::occupy) / [`vacate`](SetEmplacement::vacate).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetEmplacement {
    /// The emplacement terrain entity to (re)set.
    emplacement: Entity,
    /// The ganger entity the request concerns — seated on occupy, unseated on vacate.
    ganger:      Entity,
    /// The target state to set the emplacement to.
    state:       EmplacementState,
}

impl SetEmplacement {
    /// Build a set-state request for `emplacement` concerning `ganger` → `state` (the explicit
    /// occupy-or-vacate form).
    #[must_use]
    pub const fn new(emplacement: Entity, ganger: Entity, state: EmplacementState) -> Self {
        Self {
            emplacement,
            ganger,
            state,
        }
    }

    /// Build an OCCUPY request — seat `ganger` in `emplacement` (a convenience over
    /// [`new`](SetEmplacement::new)).
    #[must_use]
    pub const fn occupy(emplacement: Entity, ganger: Entity) -> Self {
        Self::new(emplacement, ganger, EmplacementState::Occupied)
    }

    /// Build a VACATE request — unseat `ganger` from `emplacement` (a convenience over
    /// [`new`](SetEmplacement::new)).
    #[must_use]
    pub const fn vacate(emplacement: Entity, ganger: Entity) -> Self {
        Self::new(emplacement, ganger, EmplacementState::Vacant)
    }

    /// The targeted emplacement entity.
    #[must_use]
    pub const fn emplacement(self) -> Entity {
        self.emplacement
    }

    /// The ganger entity the request concerns.
    #[must_use]
    pub const fn ganger(self) -> Entity {
        self.ganger
    }

    /// The target state.
    #[must_use]
    pub const fn state(self) -> EmplacementState {
        self.state
    }
}

/// Apply every buffered [`SetEmplacement`]: flip the emplacement's [`EmplacementState`] and
/// maintain the occupant's cover band + occupant record to match (GTW-543).
///
/// For each message the emplacement's [`EmplacementState`] + its [`TerrainCell`] are fetched
/// from the `emplacements` query (a `SetEmplacement` for a non-emplacement / despawned entity —
/// no `EmplacementState` — is skipped, panic-free). Then, when the requested state DIFFERS from
/// the current one (idempotent — re-setting the same state writes nothing, so it never trips a
/// spurious `Changed`, and re-occupying an ALREADY-occupied emplacement is the no-force-eject
/// no-op):
///
/// - `→ Occupied`: write [`EmplacementState::Occupied`], insert an
///   [`EmplacementOccupant`]`(ganger)` via `Commands`, FORCE the occupant's silhouette band
///   to [`HeightBand::High`](crate::cover::HeightBand::High) at the emplacement cell (`set_occupant_band(cell, Some(High))`) —
///   the occupant reads as HIGH cover — and SPAWN the mounted weapon: resolve the emplacement's
///   [`MountedWeaponKey`](super::MountedWeaponKey) against the [`WeaponRegistry`], spawn its
///   [`WeaponBundle`](crate::weapon::WeaponBundle) related to `ganger` via
///   [`WieldedBy`](crate::weapon::WieldedBy) + the [`MountedWeapon`](crate::weapon::MountedWeapon)
///   marker, and record the spawned entity in a [`MountedWeaponEntity`](super::MountedWeaponEntity)
///   on the emplacement (a missing registry / unresolved key spawns no mount — fail-closed).
/// - `→ Vacant`: write [`EmplacementState::Vacant`], remove the [`EmplacementOccupant`] via
///   `Commands`, RESTORE the occupant's band from its [`Stance`] silhouette
///   ([`silhouette_band`], defaulting to standing → HIGH when the ganger carries no `Stance`), and
///   DESPAWN the recorded [`MountedWeaponEntity`](super::MountedWeaponEntity) (the
///   [`WieldedBy`](crate::weapon::WieldedBy) edge goes with it), reverting the occupant to its own
///   carried gun and removing the record.
///
/// `bevy-traps.md` #7: `MessageReader` / `Query` / `Res` / `ResMut` / `Commands` — no
/// `&mut World`. `bevy-traps.md` #4: a buffered `MessageReader`, drained every run. The
/// [`WeaponRegistry`] is taken `Option<Res>` so a focused harness that opens the toggle WITHOUT
/// the Load-flow registry does not panic (`bevy-traps.md` #1) — it just spawns no mount.
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
            // Not an emplacement entity (no EmplacementState / despawned) — skip, panic-free.
            continue;
        };
        if *state == request.state() {
            // Idempotent: already in the requested state — no flip, no band edit, no spurious
            // Changed. Occupying an already-Occupied emplacement is the no-force-eject no-op.
            continue;
        }
        *state = request.state();
        let ganger = request.ganger();
        let key = **cell;
        if request.state().is_occupied() {
            // Occupy: record the occupant + force its band to HIGH so it reads as HIGH cover.
            commands
                .entity(request.emplacement())
                .insert(EmplacementOccupant::new(ganger));
            grid.set_occupant_band(key, Some(silhouette_band(StanceKind::Standing)));
            // Spawn the mounted weapon on the occupant (WieldedBy + a MountedWeapon marker),
            // resolving MountedWeaponKey against the WeaponRegistry, and record its entity so the
            // vacate branch can despawn exactly it. A missing registry / unresolved key spawns no
            // mount (fail-closed — the band-force + occupant record still stand).
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
            // Vacate: drop the occupant record + restore its band from its stance silhouette
            // (default standing → HIGH when the ganger has no Stance).
            commands
                .entity(request.emplacement())
                .remove::<EmplacementOccupant>();
            let stance = stances
                .get(ganger)
                .map_or(StanceKind::Standing, |stance| **stance);
            grid.set_occupant_band(key, Some(silhouette_band(stance)));
            // Despawn the recorded mounted-weapon entity (the WieldedBy edge goes with it) so the
            // occupant reverts to its own carried gun, and drop the record.
            if let Some(mount) = mounted_entity {
                commands.entity(**mount).despawn();
                commands
                    .entity(request.emplacement())
                    .remove::<MountedWeaponEntity>();
            }
        }
    }
}

/// Spawn the mounted weapon for a newly-occupied emplacement on its `occupant` — resolve the
/// emplacement's [`MountedWeaponKey`] against the [`WeaponRegistry`] and, on success, spawn its
/// [`WeaponBundle`](crate::weapon::WeaponBundle) related to `occupant` via
/// [`WieldedBy`](crate::weapon::WieldedBy) + the [`MountedWeapon`](crate::weapon::MountedWeapon)
/// marker, returning the spawned weapon [`Entity`] (GTW-543).
///
/// Returns `None` — spawning nothing — when the emplacement carries no [`MountedWeaponKey`], the
/// [`WeaponRegistry`] is absent (a focused harness with no Load flow), or the key resolves to no
/// spec (fail-closed; the occupy path still applies the band-force + occupant record). The weapon
/// carries the [`MountedWeapon`](crate::weapon::MountedWeapon) marker so the ranged-firing read
/// PREFERS it while the ganger mans the mount.
///
/// GTW-549: the mounted weapon's authored `attachments` keys are resolved against the
/// [`AttachmentRegistry`] into a
/// [`PendingAttachments`](crate::weapon::PendingAttachments) marker spawned onto the weapon (an
/// EMPTY marker when it authors none / the registry is absent), which the post-spawn
/// [`apply_pending_attachments`](crate::apply_pending_attachments) system applies via
/// the [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon) extension — the
/// SAME path the `setup_battle` spawn uses. GTW-554: the resolution is slot-gated
/// ([`resolve_pending_attachments`] — the ONE shared seam): an item only fits a slot the
/// mounted weapon's spec declares, with free capacity.
fn spawn_mounted_weapon(
    commands: &mut Commands,
    occupant: Entity,
    mounted_key: Option<&MountedWeaponKey>,
    weapons: Option<&WeaponRegistry>,
    attachments: Option<&AttachmentRegistry>,
) -> Option<Entity> {
    let key = mounted_key?;
    let spec = weapons?.spec(key)?;
    // GTW-549: resolve the mounted weapon's authored attachment keys into a PendingAttachments
    // marker (empty when it authors none / the registry is absent — the fail-safe). GTW-554:
    // through the shared SLOT-GATED seam (`resolve_pending_attachments`), so a mount rejects a
    // wrong-slot / over-capacity item exactly like the setup_battle spawn. The
    // WeaponSpawnSiblings (dot / on_death) are carried into the scene by setup_battle only; a
    // directly-spawned mount composes just the bundle + relationship + pending marker.
    let pending = resolve_pending_attachments(&spec.slots, &spec.attachments, attachments);
    let (bundle, _siblings) = spec.clone().into_bundle((**key).clone());
    // Spawn the mounted weapon related to the occupant (the WieldedBy hook adds it to the
    // occupant's Wields collection) + the MountedWeapon marker the ranged-firing read prefers +
    // the PendingAttachments marker the post-spawn applier reads.
    Some(
        commands
            .spawn((bundle, WieldedBy::new(occupant), MountedWeapon, pending))
            .id(),
    )
}

/// Wires the emplacement toggle into a Bevy [`App`] (GTW-543) — registers the
/// [`SetEmplacement`] message buffer and adds [`apply_emplacement_toggle`] to [`Update`].
///
/// In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s [`SetEmplacement`] (`bevy-traps.md` #5 — an
///   unregistered buffer fails [`apply_emplacement_toggle`]'s `MessageReader` param
///   validation); and
/// - adds [`apply_emplacement_toggle`] `.in_set(`[`SimSystems::Simulate`]`)` ordered
///   `.after(`[`sync_moved_gangers`]`)` so the occupy-frame HIGH band-force is the last band
///   write, not clobbered by the same-tick stance-band publish (see the module docs).
///
/// The `SimSystems::Simulate` set is OWNED upstream (by
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)), so this
/// plugin only `.in_set`s into it — it never re-configures the set (`bevy-traps.md` #5).
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

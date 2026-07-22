//! The transient-flash lifetime newtype, its marker, and the one-shot expiry clock.

use std::time::Duration;

use bevy::prelude::*;

/// How long one transient FX flash stays on screen, in seconds.
///
/// A small, fixed pop-and-fade window so a flash reads as a momentary burst rather than a
/// persistent sprite. A `const`, NOT a domain newtype — the framework-plumbing carve-out
/// (`.claude/rules/no-bare-types.md` clause 4): a scalar fed straight to a [`Timer`]
/// duration, the same reasoning the landed `CELL_PX`-class consts use. The [`FlashTtl`]
/// domain VALUE (the live countdown) IS a newtype.
pub(super) const FLASH_SECONDS: f32 = 0.4;

/// Marker tagging every transient FX flash sprite this slice spawns.
///
/// A value-free marker (the no-bare-types marker carve-out) so [`expire_flashes`] finds
/// exactly the FX flashes — and ONLY them, never the S4 [`TerrainSprite`](crate::TerrainSprite),
/// never the S5 [`GangerSprite`](crate::GangerSprite), never the S2
/// [`WorldCamera`](crate::WorldCamera).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FxFlash;

/// The lifetime of one transient FX flash — its despawn clock.
///
/// A NAMED newtype over a [`Timer`] (no-bare-types: a transient-FX lifetime is a domain
/// value, not a bare `Timer`), [`Deref`]ing to it so a reader inspects the timer straight
/// through. THIS countdown is what makes "one-shot" observable: [`expire_flashes`] advances
/// it each `Update` and despawns the flash the moment it [`Timer::finished`]. A fresh
/// [`FlashTtl::new`] starts a `FLASH_SECONDS` one-shot ([`TimerMode::Once`]) clock.
#[derive(Component, Deref, Debug, Clone)]
pub struct FlashTtl(Timer);

impl FlashTtl {
    /// Start a fresh one-shot flash clock running for `FLASH_SECONDS`.
    ///
    /// [`TimerMode::Once`] so the timer finishes exactly once (it does not loop), which is
    /// the "one-shot" semantics [`expire_flashes`] keys its despawn on.
    #[must_use]
    pub fn new() -> Self {
        Self(Timer::from_seconds(FLASH_SECONDS, TimerMode::Once))
    }

    /// Advance this flash clock by `delta` and report whether it has now expired.
    ///
    /// Wraps [`Timer::tick`] + [`Timer::is_finished`] so the inner [`Timer`] is mutated
    /// through a named method (no `DerefMut` exposed — the lifetime is advanced ONLY here).
    /// Returns `true` once the `FLASH_SECONDS` window has elapsed (a [`TimerMode::Once`]
    /// timer stays finished once it crosses), the signal [`expire_flashes`] despawns on.
    pub fn tick(&mut self, delta: Duration) -> bool {
        self.0.tick(delta).is_finished()
    }
}

impl Default for FlashTtl {
    /// A fresh `FLASH_SECONDS` one-shot clock — same as [`FlashTtl::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// `Update` (`PresenterSystems::Overlay`): the one-shot despawn-on-expiry system.
///
/// Advances each [`FlashTtl`] by the frame [`Res<Time>`] delta ([`FlashTtl::tick`]) and
/// `Commands::entity(e).despawn()`s the flash the moment its clock finishes. THIS is what
/// makes the flashes one-shot / transient — without it, a spawned flash would linger forever.
/// It touches ONLY [`FxFlash`]-marked entities (never terrain / ganger sprites / the camera).
/// Needs no `BattleInProgress` gate — it is inert with no flashes (the query is empty), so it
/// is registered unguarded by that witness.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the despawn, [`Res<Time>`] for the
/// delta, and the `(Entity, &mut FlashTtl)` query (`With<FxFlash>`).
pub fn expire_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut FlashTtl), With<FxFlash>>,
) {
    let delta = time.delta();
    for (entity, mut ttl) in &mut flashes {
        if ttl.tick(delta) {
            commands.entity(entity).despawn();
        }
    }
}

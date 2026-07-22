//! The flight component: the [`ShotProjectile`] marker and the constant-velocity
//! [`ProjectileTravel`] state machine.

use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{Cell, Level},
    resolve_and_apply::HitReport,
    weapon::DamageType,
};

use super::super::{fct::ClassifiedPop, tuning::ProjectileVelocity};

/// Marker tagging every traveling projectile sprite this slice spawns.
///
/// A value-free marker (the no-bare-types marker carve-out) so
/// [`advance_projectiles`](super::advance::advance_projectiles)
/// finds exactly the GTW-306 projectiles — never the FX flashes
/// ([`FxFlash`](super::super::FxFlash)),
/// never terrain / ganger sprites / the camera.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShotProjectile;

/// One projectile's flight — its CONSTANT-VELOCITY travel from the muzzle world point
/// toward the target world point, after a staggered launch delay.
///
/// A NAMED grouping component (not a bare tuple): `from` /
/// `to` are the endpoints (world-space, the
/// [`Vec3`](bevy::math::Vec3) inner is the only carve-out the no-bare-types rule
/// allows — framework geometry the [`Transform`] consumes directly), `damage` is the
/// shot's [`DamageType`] (carried so the arrival [`PendingImpact`](super::pending::PendingImpact)
/// knows which
/// 3-frame impact to play), `launch` is the one-shot LAUNCH-DELAY clock that holds the
/// round at the muzzle (invisible) until the volley's stagger step elapses,
/// `velocity` is the bolt's constant flight speed (the hot-reloadable
/// [`ProjectileVelocity`] CAPTURED from [`FxTuning`](super::super::tuning::FxTuning) at
/// spawn, so a live `.ron` edit
/// re-tunes the next shot), and `traveled` is the distance (world px) the bolt has
/// covered along the `from → to` ray so far — advanced by `*velocity × delta` each
/// frame once launched. The launch timer + traveled distance mutate ONLY through
/// [`advance`](ProjectileTravel::advance) (no `DerefMut`).
///
/// GTW-327 (slice 2): the bolt ALSO carries this round's classified floating-combat-text
/// `pops` (the `ClassifiedPop` list
/// `classify_report` built from the shot's
/// [`HitReport`](gdtf_battle_sim::resolve_and_apply::HitReport) — empty for a clean miss) + the `anchor`
/// `(cell, level)` the pops sit on (`anchor_cell` of the
/// hit ganger at the SHOT, not the
/// impact cell), so each shot's numbers ride its own staggered flight and appear when THAT
/// shot's impact lands — handed to the arrival [`PendingImpact`](super::pending::PendingImpact).
#[derive(Component, Debug, Clone)]
pub struct ProjectileTravel {
    /// The muzzle world point the flight starts at (`traveled = 0`).
    from:     Vec3,
    /// The target world point the flight ends at (`traveled = distance(from, to)`).
    to:       Vec3,
    /// The shot's damage type — handed to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact) so FX-B
    /// plays the matching 3-frame impact animation.
    damage:   DamageType,
    /// The one-shot LAUNCH-DELAY clock (this round's read-order index × the
    /// hot-reloadable [`InterShotSeconds`](super::super::tuning::InterShotSeconds)). Until it
    /// finishes the round is held INVISIBLE at the muzzle so a multi-round volley
    /// animates shot-by-shot; once it finishes the bolt starts accumulating `traveled`.
    launch:   Timer,
    /// The bolt's constant flight speed (world px/sec) — the hot-reloadable
    /// [`ProjectileVelocity`] this round CAPTURED from
    /// [`FxTuning`](super::super::tuning::FxTuning) at spawn, so a live
    /// `.ron` edit re-tunes subsequently fired shots without a rebuild.
    velocity: ProjectileVelocity,
    /// Distance (world px) flown along the `from → to` ray so far — grows by
    /// `*velocity × delta` each launched frame; arrival is when it reaches the full
    /// `from → to` distance.
    traveled: f32,
    /// This shot's classified floating-combat-text pops (GTW-327) — handed to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact) so
    /// [`animate_impact`](super::super::impact::animate_impact) spawns them
    /// when THIS shot's impact lands (staggered with the bolt). Empty for a clean miss.
    pops:     Vec<ClassifiedPop>,
    /// The `(cell, level)` this shot's pops anchor over (GTW-327) — captured at the SHOT
    /// ([`anchor_cell`](super::super::fct::anchor_cell), the hit ganger's cell, not the
    /// impact cell) and threaded through so
    /// the pops sit on the body that was hit.
    anchor:   (Cell, Level),
    /// The firing entity (GTW-328) — threaded through so the arrival
    /// [`PendingImpact`](super::pending::PendingImpact) can name the shooter when
    /// [`animate_impact`](super::super::impact::animate_impact)
    /// emits the shared [`ShotImpactResolved`](super::super::impact::ShotImpactResolved)
    /// signal the
    /// combat log keys its outcome lines off (the [`Entity`] is framework plumbing, the
    /// no-bare-types carve-out). It rides the bolt so the signal fires at THIS shot's staggered
    /// impact, not on the fire frame.
    shooter:  Entity,
    /// This shot's already-computed hit report (GTW-328) — the sim's verdict, threaded through to
    /// the arrival [`PendingImpact`](super::pending::PendingImpact) so the impact-resolved
    /// signal carries the data the combat log
    /// classifies a shot outcome from. [`None`] carries no verdict — the log renders no outcome
    /// line for it (GTW-559); every fired volley round carries `Some`, a clean miss included.
    report:   Option<HitReport>,
}

impl ProjectileTravel {
    /// Start a fresh projectile flight from `from` toward `to` carrying `damage`, flying at
    /// `velocity`, after a `launch_delay` hold at the muzzle, carrying this shot's classified
    /// floating-combat-text `pops` anchored over `anchor`.
    ///
    /// `velocity` is the hot-reloadable [`ProjectileVelocity`] the caller READ from the
    /// resident [`FxTuning`](super::super::tuning::FxTuning) resource — CAPTURED here so a
    /// later `.ron` edit re-tunes the
    /// NEXT shot rather than mid-flight ones. `launch_delay` is this round's stagger offset
    /// (its read-order index × the tuning's
    /// [`InterShotSeconds`](super::super::tuning::InterShotSeconds)); a
    /// [`Duration::ZERO`](std::time::Duration::ZERO) delay launches at once (round 0 / a
    /// single shot — a zero-duration `Once` timer is already finished). The launch hold is a
    /// [`TimerMode::Once`] clock; once it finishes the bolt flies at `*velocity` (px/sec)
    /// until it has covered the whole `from → to` distance, at which point it has arrived and
    /// is despawned.
    ///
    /// `pops` (GTW-327) is this round's [`ClassifiedPop`] list
    /// ([`classify_report`](super::super::fct::classify_report) of the
    /// shot's report — empty for a clean miss) and `anchor` the `(cell, level)` they sit on
    /// ([`anchor_cell`](super::super::fct::anchor_cell)); they are carried through to the
    /// arrival [`PendingImpact`](super::pending::PendingImpact) so the
    /// numbers appear when THIS shot's impact lands rather than on the drain frame.
    ///
    /// `pub(in crate::actors::fx)`: it takes the FX-internal [`ClassifiedPop`], so it stays sealed to
    /// the FX layer (the `spawn_shot_projectiles` caller + the in-crate flight tests construct
    /// it; nothing outside `crate::fx` needs to build a flight).
    ///
    /// `shooter` + `report` (GTW-328) ride through to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact) so
    /// [`animate_impact`](super::super::impact::animate_impact) can emit the shared
    /// [`ShotImpactResolved`](super::super::impact::ShotImpactResolved) signal — naming the
    /// shooter and
    /// carrying its verdict — when THIS shot's staggered impact lands (the combat log keys its
    /// outcome lines off that signal, not the fire-frame `ShotFired` drain).
    #[expect(
        clippy::too_many_arguments,
        reason = "each is a distinct flight datum: the two endpoints, the damage type, the \
                  velocity, the stagger launch delay, the GTW-327 pops + anchor, and the GTW-328 \
                  shooter + report threaded to the impact signal — the ctor IS the bundle"
    )]
    #[must_use]
    pub(in crate::actors::fx) fn new(
        from: Vec3,
        to: Vec3,
        damage: DamageType,
        velocity: ProjectileVelocity,
        launch_delay: std::time::Duration,
        pops: Vec<ClassifiedPop>,
        anchor: (Cell, Level),
        shooter: Entity,
        report: Option<HitReport>,
    ) -> Self {
        Self {
            from,
            to,
            damage,
            launch: Timer::new(launch_delay, TimerMode::Once),
            velocity,
            traveled: 0.0,
            pops,
            anchor,
            shooter,
            report,
        }
    }

    /// The full `from → to` flight distance, in world px.
    #[must_use]
    fn distance(&self) -> f32 {
        self.from.distance(self.to)
    }

    /// Advance the round by `delta` and report whether its flight has now ARRIVED.
    ///
    /// The LAUNCH-DELAY hold ticks first: while it is unfinished the round stays parked at
    /// the muzzle (no distance accrues) and this returns `false`. Once the launch delay
    /// finishes, the bolt accumulates `*velocity × delta` world px of travel (the captured
    /// hot-reloadable [`ProjectileVelocity`]); this returns `true` only once `traveled` has
    /// reached the full `from → to` distance — the signal
    /// [`advance_projectiles`](super::advance::advance_projectiles) despawns +
    /// spawns the [`PendingImpact`](super::pending::PendingImpact) on. Wraps the inner
    /// [`Timer`] + distance so they mutate
    /// through a named method (no `DerefMut`).
    pub fn advance(&mut self, delta: std::time::Duration) -> bool {
        if !self.launch.tick(delta).is_finished() {
            // Still parked at the muzzle — hold this round until its stagger step elapses.
            return false;
        }
        self.traveled = self.velocity.mul_add(delta.as_secs_f32(), self.traveled);
        self.traveled >= self.distance()
    }

    /// Whether this round's launch delay has elapsed — i.e. it has LEFT the muzzle and
    /// should now be drawn. While `false` the round is held invisible at the muzzle so the
    /// volley animates shot-by-shot.
    #[must_use]
    pub fn launched(&self) -> bool {
        self.launch.is_finished()
    }

    /// The current travel fraction `t ∈ [0, 1]` (distance flown / total distance) — the
    /// flight progress the projectile's world position interpolates by. Reads `0` while the
    /// round is still parked at the muzzle (no distance flown yet); a zero-length flight
    /// (muzzle == target) reads `1` (already arrived) rather than dividing by zero.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        let distance = self.distance();
        if distance <= f32::EPSILON {
            return 1.0;
        }
        (self.traveled / distance).clamp(0.0, 1.0)
    }

    /// The projectile's CURRENT world position — `from.lerp(to, t)` at the travel
    /// fraction, the constant-velocity travel point (NEVER a stretch/scale).
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.from.lerp(self.to, self.fraction())
    }

    /// The arrival (target) world point — the flight terminus (`to`).
    #[must_use]
    pub const fn arrival(&self) -> Vec3 {
        self.to
    }

    /// The shot's damage type — handed to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact).
    #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.damage
    }

    /// This shot's classified floating-combat-text pops (GTW-327), moved out at arrival so the
    /// [`PendingImpact`](super::pending::PendingImpact) owns them (the bolt is despawned the
    /// same frame, so the [`Vec`] is not
    /// needed on it afterward). Empty for a clean miss.
    #[must_use]
    pub(super) fn take_pops(&mut self) -> Vec<ClassifiedPop> {
        std::mem::take(&mut self.pops)
    }

    /// The `(cell, level)` this shot's pops anchor over (GTW-327) — handed to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact).
    #[must_use]
    pub(super) const fn anchor(&self) -> (Cell, Level) {
        self.anchor
    }

    /// The firing entity (GTW-328) — handed to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact) so the
    /// impact-resolved signal names the shooter.
    #[must_use]
    pub(super) const fn shooter(&self) -> Entity {
        self.shooter
    }

    /// This shot's hit report (GTW-328) — handed to the arrival
    /// [`PendingImpact`](super::pending::PendingImpact) so the
    /// impact-resolved signal carries the verdict the combat log classifies. Cloned (the
    /// [`HitReport`] is non-`Copy` since GTW-438 — it carries the rolled injury).
    #[must_use]
    pub(super) fn report(&self) -> Option<HitReport> {
        self.report.clone()
    }
}

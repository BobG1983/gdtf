//! The GTW-359 general sprite-movement TWEEN (E7 · GTW-12k, AC3 / OQ-5 — subsumes
//! GTW-361): a per-sprite glide that interpolates a ganger sprite's [`Transform`]
//! toward its latest cell rather than SNAPPING it.
//!
//! # Why a re-targeting glide (the GTW-355 cadence note)
//!
//! The sim's [`advance_walk`](gdtf_battle_sim) writes ONE discrete
//! [`Position`](gdtf_battle_sim::Position) per TICK during a committed walk, so the
//! presenter sees a SEQUENCE of [`Changed<Position>`] over frames at ~tick rate. A
//! fixed-duration per-step tween would DESYNC (the sim outruns it and the sprite would
//! fall behind, then snap). Instead this is a RE-TARGETING glide: on each
//! [`Changed<Position>`] the mover sets the tween's source to the sprite's CURRENT
//! (possibly mid-glide) translation and its target to the new cell world position,
//! restarting the clock. So if the sim outruns the tween, the sprite continuously glides
//! toward the LATEST cell and never snaps — and it never GATES the sim (the sim's
//! position is authoritative; the tween only smooths the view of it).
//!
//! FLAG (the C4 cadence trailing): the walk visually trails the sim by up to one
//! step-duration. A deliberately PACED walk (one step per animation interval) is a
//! GTW-355 [`advance_walk`](gdtf_battle_sim) follow-up — out of scope here.
//!
//! # The `ImpactAnimation` / `FloatingCombatText` precedent
//!
//! Manual lerp (no tween crate): a per-component [`Timer`] (the
//! [`ImpactAnimation`](crate::fx) / [`FloatingCombatText`](crate::FloatingCombatText)
//! precedent), ticked by [`Res<Time>`], drives a [`Vec3::lerp`] from source to target.
//! It mutates the sprite [`Transform`] IN PLACE (never despawn / respawn — C5). Covers
//! planar AND cross-storey transitions (the GENERAL per-step glide).

use std::time::Duration;

use bevy::prelude::*;

use super::sprite_map::GangerSprite;

/// The fixed glide duration of ONE sprite-movement tween, in seconds.
///
/// Framework plumbing — a timing scalar fed straight to a [`Timer`], not a domain
/// quantity (the `CELL_PX`-class const carve-out, the
/// [`ImpactFrameSeconds`](crate::ImpactFrameSeconds) precedent). Short (one fifth of a
/// second) so a move reads as a quick glide, not a slow drift; a re-target restarts this
/// clock from the sprite's current translation, so a fast sim walk continuously glides
/// toward the latest cell (the cadence note above).
const TWEEN_SECONDS: f32 = 0.2;

/// One playing sprite-movement tween — the glide source / target world positions and the
/// per-glide clock.
///
/// A NAMED grouping component (not a bare tuple): `source` is the world translation the
/// glide STARTS from (the sprite's translation when the tween was last [`retarget`](SpriteTween::retarget)ed,
/// possibly mid-glide), `target` is the destination cell's world translation, and `clock`
/// is the [`TimerMode::Once`] glide clock [`advance_sprite_tweens`] ticks. The inner
/// fields mutate ONLY through [`retarget`](SpriteTween::retarget) /
/// [`advance`](SpriteTween::advance) (no `DerefMut`); construction is through
/// [`settled`](SpriteTween::settled). The `Vec3` source / target are presenter-internal
/// world-space draw positions (the `cell_to_world_layered` projection output, the
/// `CELL_PX`-class framework-plumbing carve-out), not sim domain values.
#[derive(Component, Debug, Clone)]
pub struct SpriteTween {
    /// The world translation the current glide starts from.
    source: Vec3,
    /// The world translation the current glide ends at (the latest cell's projection).
    target: Vec3,
    /// The one-shot glide clock; its fraction drives the source→target [`Vec3::lerp`].
    clock:  Timer,
}

impl SpriteTween {
    /// A SETTLED tween at `position` — source == target, the clock finished, so it
    /// produces no motion until a [`retarget`](SpriteTween::retarget).
    ///
    /// Seeded onto a ganger sprite at spawn so the move path always finds a tween to
    /// re-target (rather than special-casing the first move). The clock is constructed
    /// already finished (ticked past its duration), so [`advance`](SpriteTween::advance)
    /// holds the sprite at `position` until the first move retargets it.
    #[must_use]
    pub fn settled(position: Vec3) -> Self {
        let mut clock = Timer::from_seconds(TWEEN_SECONDS, TimerMode::Once);
        // Finish the clock immediately: a freshly-spawned sprite is already AT its cell,
        // so the tween produces no motion until a move retargets it.
        clock.tick(Duration::from_secs_f32(TWEEN_SECONDS));
        Self {
            source: position,
            target: position,
            clock,
        }
    }

    /// Re-target the glide: start a fresh glide from `from` (the sprite's CURRENT,
    /// possibly mid-glide, translation) to `to` (the latest cell's world position),
    /// restarting the clock.
    ///
    /// The contract's RE-TARGET (AC3 / OQ-5): a move sets `from` to where the sprite
    /// actually IS this frame, not to the previous target, so a sim that outruns the
    /// tween keeps the sprite gliding continuously toward the latest cell — it never
    /// snaps. Restarting the clock from the current position is what makes the glide
    /// seamless across a rapid sequence of [`Changed<Position>`].
    pub fn retarget(&mut self, from: Vec3, to: Vec3) {
        self.source = from;
        self.target = to;
        self.clock.reset();
    }

    /// Advance the glide clock by `delta` and report the interpolated translation.
    ///
    /// Ticks the one-shot clock and returns `source + (target - source) * fraction`.
    /// While the glide is running this is an INTERMEDIATE point strictly between source
    /// and target (the AC3 "not snapped" property); the moment the clock FINISHES it
    /// returns the `target` EXACTLY (a finished clock short-circuits, so a settled tween —
    /// source == target — and a completed glide both land on the target with no
    /// floating-point drift, which an exact equality on the destination world position
    /// relies on). The `source + delta * fraction` form is exact at the endpoints (unlike
    /// [`Vec3::lerp`]'s `source * (1 - f) + target * f`, which can round a tick away from
    /// the target). Wraps [`Timer::tick`] so the inner [`Timer`] mutates only here (no
    /// `DerefMut`).
    pub fn advance(&mut self, delta: Duration) -> Vec3 {
        if self.clock.tick(delta).is_finished() {
            // Settled / glide complete: land exactly on the target (no FP drift).
            return self.target;
        }
        let fraction = self.clock.fraction();
        self.source + (self.target - self.source) * fraction
    }
}

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems),
/// `.after(move_ganger_sprites)`): glide every ganger sprite's [`Transform`] toward its
/// tween target.
///
/// For each [`GangerSprite`] carrying a [`SpriteTween`], it ticks the tween by the frame
/// [`Res<Time>`] delta ([`SpriteTween::advance`]) and writes the interpolated translation
/// to the sprite's [`Transform`] IN PLACE (never despawn / respawn — C5). A settled tween
/// (source == target, or its clock finished) holds the sprite at its cell, so a stationary
/// ganger does no visible work. It runs `.after(move_ganger_sprites)` so a same-frame
/// re-target (the move path) is reflected this frame (the glide starts immediately).
///
/// This is the GENERAL per-step glide (AC3 / OQ-5): it covers planar AND cross-storey
/// moves, because [`move_ganger_sprites`](super::move_ganger_sprites) re-targets the tween
/// for EVERY [`Changed<Position>`] regardless of storey, and the [`Visibility`] hard-cut
/// (the C3 handoff) stays in `move_ganger_sprites` — this system only moves the
/// [`Transform`].
///
/// Param-only (`bevy-traps.md` #7): [`Res<Time>`] for the per-frame delta and a
/// `Query<(&mut Transform, &mut SpriteTween), With<GangerSprite>>` for the in-place glide.
pub fn advance_sprite_tweens(
    time: Res<Time>,
    mut tweens: Query<(&mut Transform, &mut SpriteTween), With<GangerSprite>>,
) {
    let delta = time.delta();
    for (mut transform, mut tween) in &mut tweens {
        transform.translation = tween.advance(delta);
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use bevy::math::Vec3;

    use super::{SpriteTween, TWEEN_SECONDS};

    /// A settled tween holds its position — source == target, clock finished — so it
    /// produces NO motion before a retarget (a stationary ganger's sprite never drifts).
    #[test]
    fn settled_tween_holds_position() {
        let at = Vec3::new(3.0, -4.0, 0.1);
        let mut tween = SpriteTween::settled(at);
        // A full-duration tick still yields exactly `at` (no motion, the clock is done).
        let held = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS));
        assert!(
            held.distance(at) < f32::EPSILON,
            "a settled tween must hold its position (got {held:?}, expected {at:?})",
        );
    }

    /// A retargeted tween, ticked HALF its duration, lands STRICTLY BETWEEN source and
    /// target (the AC3 "intermediate, not snapped" property) — then settles exactly on
    /// the target once the clock finishes.
    #[test]
    fn retargeted_tween_is_intermediate_then_settles() {
        let source = Vec3::new(0.0, 0.0, 0.1);
        let target = Vec3::new(10.0, 0.0, 0.1);
        let mut tween = SpriteTween::settled(source);
        tween.retarget(source, target);

        // Half the glide duration: the sprite is strictly between the endpoints.
        let mid = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS / 2.0));
        assert!(
            mid.x > source.x && mid.x < target.x,
            "a mid-glide tween must be strictly between source ({}) and target ({}), got {}",
            source.x,
            target.x,
            mid.x,
        );

        // Finishing the clock lands exactly on the target.
        let done = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS));
        assert!(
            done.distance(target) < f32::EPSILON,
            "a finished tween must settle exactly on the target (got {done:?}, expected \
             {target:?})",
        );
    }

    /// Re-targeting MID-glide starts a fresh glide from the CURRENT translation (not the
    /// previous target) — the contract's never-snap property when the sim outruns the
    /// tween. After a mid-glide retarget the next tick is between the MID point and the
    /// new target, never jumping back to the old source or snapping to the new target.
    #[test]
    fn retarget_mid_glide_starts_from_current_position() {
        let a = Vec3::new(0.0, 0.0, 0.1);
        let b = Vec3::new(10.0, 0.0, 0.1);
        let mut tween = SpriteTween::settled(a);
        tween.retarget(a, b);

        // Glide partway toward b.
        let mid = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS / 2.0));
        assert!(mid.x > a.x && mid.x < b.x, "partway toward b");

        // The sim outran the tween: a new cell `c` arrives — y rises from the mid point,
        // x held at the mid (so the only motion is along y, isolating the never-snap test).
        // Re-target FROM the current mid point: the move path passes the sprite's live
        // translation as the source, so the fresh glide starts where the sprite IS.
        let c = Vec3::new(mid.x, 10.0, 0.1);
        tween.retarget(mid, c);
        let after = tween.advance(Duration::from_secs_f32(TWEEN_SECONDS / 4.0));
        // The fresh glide starts at `mid` and heads toward `c` (y rising from mid.y == 0,
        // never jumping back to `a`'s y or snapping to `c`'s y).
        assert!(
            after.y > mid.y && after.y < c.y,
            "after a mid-glide retarget the glide must continue from the current position \
             ({mid:?}) toward the new target ({c:?}), got {after:?}",
        );
        // The x stays at the mid point (the new target shares the mid's x) — no snap-back
        // to `a`'s x (0.0), confirming the glide re-anchored on the live position.
        assert!(
            (after.x - mid.x).abs() < 1.0e-4,
            "the x stays at the mid point (no snap-back to a's x), got {} vs mid {}",
            after.x,
            mid.x,
        );
    }
}

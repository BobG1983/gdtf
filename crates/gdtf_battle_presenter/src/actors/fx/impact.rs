//! The GTW-306 3-FRAME ANIMATED impact FX — FX-B's slice.
//!
//! This module is the stub FX-A created so FX-B fills only the body (no `mod.rs`
//! edit collision): FX-A already declares + registers [`animate_impact`] in
//! [`fx::mod`](super) and wires it into the `PresenterSystems::Draw` band, and
//! defines the [`PendingImpact`](super::projectile::PendingImpact) SEAM
//! [`advance_projectiles`](super::projectile::advance_projectiles) spawns at a
//! projectile's arrival point (carrying the arrival world position + the shot's
//! [`DamageType`](gdtf_battle_sim::DamageType)).
//!
//! FX-B's job (this file): turn each arrived [`PendingImpact`] into a 3-FRAME
//! impact ANIMATION at its [`at`](super::projectile::PendingImpact::at) point —
//! play the damage type's three impact tiles
//! ([`EffectRoles::fx_for`](super::roles::EffectRoles::fx_for)`(damage).impact`,
//! the solid-burst → open-ring → breaking-ring sequence the sheet authors in row
//! cols 8,9,10) in order, each held for a few frames, then despawn. The muzzle
//! flash + traveling projectile stay sane in their own systems (FX-A); this slice
//! ONLY animates the impact.
//!
//! Pure VIEW (ADR-0001): it READS the [`PendingImpact`] seam + the data-driven
//! [`EffectRoles`] table and draws sprites; it never writes the sim. Param-only
//! throughout (`bevy-traps.md` #7).

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{message::MessageWriter, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{DamageType, HitReport};

use super::{
    projectile::spawn_pops_at_anchor,
    readers::fx_sprite_scaled,
    roles::{EffectRoles, IMPACT_FRAME_COUNT},
    tuning::{FxTuning, ImpactFrameSeconds},
};
use crate::TopDownAtlases;

/// A per-shot SHOT-IMPACT-RESOLVED signal — emitted (GTW-328) the instant each shot's
/// [`PendingImpact`](super::projectile::PendingImpact) is consumed in [`animate_impact`], i.e.
/// when the staggered bolt has flown and its impact lands.
///
/// This is the SHARED presenter-side per-shot impact moment the firing FX already keys off
/// (the floating-combat-text pops spawn here, GTW-327) — surfaced as a buffered
/// [`Message`](bevy::ecs::message::Message) so a downstream consumer can react at the SAME
/// staggered cadence. The combat-text LOG (`gdtf_app`) drains it to build a shot-outcome line PER
/// IMPACT (instead of dumping a whole volley's lines on the `ShotFired`-drain frame), and GTW-331
/// (death-despawn at impact) will reuse it. It carries exactly what a downstream needs to name +
/// classify the shot: the firing [`Entity`] and the sim's already-computed verdict.
///
/// Pure VIEW (ADR-0001): it is emitted from the presenter's own impact-resolution timing over data
/// the sim already produced (the [`HitReport`]); it adds NO sim plumbing and never writes the sim.
/// The [`shooter`](ShotImpactResolved::shooter) [`Entity`] is framework plumbing (the
/// no-bare-types carve-out), and [`report`](ShotImpactResolved::report) is the sim's own value type
/// — the consumer resolves the entity to a name + reuses the report through the shared classifier.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShotImpactResolved {
    /// The firing entity whose shot just impacted — the consumer resolves it to a display name.
    pub shooter: Entity,
    /// The sim's already-computed verdict for this shot ([`HitReport`]: damage / wound / DOWN /
    /// DEAD, or a no-effect miss). [`None`] for a geometry-only round (read as a miss). Reused
    /// through the shared classifier — never recomputed.
    pub report:  Option<HitReport>,
}

/// The per-frame UNIFORM draw scale of the impact strip, frame-by-frame (GTW-306 V3 fix).
///
/// The sheet's impact strip is an expanding shockwave (dense burst → open ring → breaking
/// ring), but at 1× the three tiles are a similar size, so the expansion barely reads in a
/// captured frame. Drawing each successive frame a little LARGER — uniformly, the SAME factor
/// on both axes (never a one-axis stretch) — makes the burst visibly GROW into a ring, so the
/// impact reads as a 3-frame animation at the arrival point rather than one static blob. One
/// entry per [`IMPACT_FRAME_COUNT`] frame, in play order. A `const` table of uniform
/// multipliers (framework plumbing fed to `custom_size`), not a domain value.
const IMPACT_FRAME_SCALES: [f32; IMPACT_FRAME_COUNT] = [1.1, 1.5, 1.9];

/// The UNIFORM draw scale for impact frame `frame` (GTW-306 V3 fix).
///
/// Looks up [`IMPACT_FRAME_SCALES`]`[frame]`, falling back to the LAST entry for an
/// out-of-range frame (a short authored strip degrades to the largest ring rather than
/// snapping to 1×). Keeps the burst → ring growth data-table-driven and panic-free.
fn impact_frame_scale(frame: usize) -> f32 {
    IMPACT_FRAME_SCALES
        .get(frame)
        .copied()
        .or_else(|| IMPACT_FRAME_SCALES.last().copied())
        .unwrap_or(1.0)
}

/// One playing impact animation — which of the 3 frames is showing, its per-frame
/// clock, and the damage type that selects the strip.
///
/// A NAMED grouping component (not a bare tuple): `damage` is the shot's
/// [`DamageType`] (so each frame redraws the matching impact tile from
/// [`EffectRoles::fx_for`](super::roles::EffectRoles::fx_for)), `frame` is the
/// 0-based index into the `IMPACT_FRAME_COUNT`-long impact strip currently on
/// screen, `frame_seconds` is the per-frame hold this animation CAPTURED from the
/// hot-reloadable [`FxTuning`] at spawn (so a live `.ron` edit re-tunes the next
/// impact's pacing), and `clock` is the per-frame [`Timer`] [`animate_impact`] ticks.
/// The inner timer + frame mutate ONLY through
/// [`advance`](ImpactAnimation::advance) (no `DerefMut`); construction is through
/// [`new`](ImpactAnimation::new).
#[derive(Component, Debug, Clone)]
pub struct ImpactAnimation {
    /// The shot's damage type — picks which color row's 3-frame impact strip plays.
    damage:        DamageType,
    /// The 0-based index of the impact frame currently shown (`0..IMPACT_FRAME_COUNT`).
    frame:         usize,
    /// The per-frame hold (seconds) this impact CAPTURED from the hot-reloadable
    /// [`FxTuning`] at spawn — the duration each `clock` runs for.
    frame_seconds: ImpactFrameSeconds,
    /// The per-frame hold clock (`*frame_seconds`); each finish steps `frame`.
    clock:         Timer,
}

/// What [`ImpactAnimation::advance`] reports after a tick — the animation either
/// keeps playing (the caller redraws the current frame's tile), or has finished
/// its last frame (the caller despawns it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImpactStep {
    /// The animation advanced to (or held on) frame `index` — redraw that tile.
    Showing(usize),
    /// The last impact frame has finished playing — despawn the animation entity.
    Finished,
}

impl ImpactAnimation {
    /// Start a fresh impact animation for `damage`, holding each frame for
    /// `frame_seconds`, showing frame `0`.
    ///
    /// `frame_seconds` is the hot-reloadable [`ImpactFrameSeconds`] the caller READ from
    /// the resident [`FxTuning`] resource — CAPTURED here so a later `.ron` edit re-tunes
    /// the NEXT impact's pacing rather than a playing one. A [`TimerMode::Once`] per-frame
    /// clock runs for `*frame_seconds`; each time it finishes
    /// [`advance`](ImpactAnimation::advance) steps to the next frame (resetting the clock)
    /// until the last frame elapses.
    #[must_use]
    pub fn new(damage: DamageType, frame_seconds: ImpactFrameSeconds) -> Self {
        Self {
            damage,
            frame: 0,
            frame_seconds,
            clock: Timer::from_seconds(*frame_seconds, TimerMode::Once),
        }
    }

    /// The damage type whose 3-frame impact strip this animation plays.
    #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.damage
    }

    /// Advance the per-frame clock by `delta`, stepping to the next impact frame
    /// when the hold elapses, and report the resulting [`ImpactStep`].
    ///
    /// While the current frame's `*frame_seconds` hold (the captured hot-reloadable
    /// [`ImpactFrameSeconds`]) has not elapsed it returns [`ImpactStep::Showing`] for
    /// the SAME frame (no redraw change). On the frame the clock finishes it steps
    /// `frame` forward and resets the clock: if a next frame exists it returns
    /// [`ImpactStep::Showing`] for it (the caller swaps the sprite to that tile); once
    /// the LAST frame (`IMPACT_FRAME_COUNT - 1`) has finished it returns
    /// [`ImpactStep::Finished`] (the caller despawns). Wraps [`Timer::tick`] so the
    /// inner [`Timer`] + frame mutate only here (no `DerefMut`).
    fn advance(&mut self, delta: std::time::Duration) -> ImpactStep {
        if !self.clock.tick(delta).is_finished() {
            // Still holding on the current frame — keep showing it.
            return ImpactStep::Showing(self.frame);
        }
        // This frame's hold elapsed — step to the next.
        self.frame += 1;
        if self.frame >= IMPACT_FRAME_COUNT {
            return ImpactStep::Finished;
        }
        // Reset the one-shot clock for the next frame's hold (the captured tuning value).
        self.clock = Timer::from_seconds(*self.frame_seconds, TimerMode::Once);
        ImpactStep::Showing(self.frame)
    }
}

/// `Update` (`PresenterSystems::Draw`): play the 3-frame damage-type impact
/// animation — AND spawn this shot's floating-combat-text pops (GTW-327) — at each
/// arrived projectile's [`PendingImpact`](super::projectile::PendingImpact).
///
/// Two passes over the world, both `Commands`/`Query`-only (no `&mut World`,
/// `bevy-traps.md` #7):
///
/// 1. **Seed → animation + pops.** For each freshly-arrived
///    [`PendingImpact`](super::projectile::PendingImpact) FX-A's
///    [`advance_projectiles`](super::projectile::advance_projectiles) spawned, it
///    builds the FIRST impact tile of the damage type's strip
///    ([`EffectRoles::fx_for`](super::roles::EffectRoles::fx_for)`(damage).impact[0]`)
///    via [`fx_sprite_scaled`](super::readers::fx_sprite_scaled) at the impact world
///    point, spawns it with an [`ImpactAnimation`], SPAWNS the shot's classified
///    floating-combat-text pops at their anchor (GTW-327 — so each shot's numbers appear
///    when THIS shot's staggered impact lands, fanned out by per-shot
///    [`FctStackIndex`](super::fct::FctStackIndex)), EMITS the shared [`ShotImpactResolved`]
///    signal (GTW-328 — the shooter + verdict, so the combat-text LOG / GTW-331 react at this
///    same staggered moment, not on the fire frame), and DESPAWNS the seed (consumed once).
///    A missing effects sheet skips the impact SPRITE fail-closed
///    (`fx_sprite_scaled` returns [`None`]) but STILL spawns the pops (they are
///    [`Text2d`], needing no atlas) and still consumes the seed (no re-attempt pile-up).
/// 2. **Step the animations.** Each [`ImpactAnimation`] is ticked by the frame
///    [`Res<Time>`] delta ([`ImpactAnimation::advance`]); on a frame step it swaps
///    the sprite to the new frame's tile, and on the last frame finishing it
///    despawns the animation entity. A short impact strip degrades gracefully (an
///    out-of-range frame skips the redraw, the animation still despawns on finish).
///
/// The per-impact-frame hold AND the pop lifetime / rise (GTW-327) are READ from the
/// hot-reloadable [`FxTuning`] resource, so a live edit to `assets/tiles/fx_tuning.ron`
/// re-tunes the next impact's pacing + the next pop's lifetime without a rebuild.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn / sprite swap /
/// despawn, [`Res<Time>`] for the per-frame delta, [`Res<EffectRoles>`] +
/// [`Res<TopDownAtlases>`] for the data-driven tiles, [`Res<FxTuning>`] for the
/// per-frame hold + the pop lifetime / rise, the
/// [`MessageWriter<ShotImpactResolved>`](bevy::ecs::message::MessageWriter) for the GTW-328
/// per-shot impact signal, and the disjoint `PendingImpact` (seed) / `ImpactAnimation` (playing)
/// queries. Its registration gates on `BattleInProgress` + `EffectRoles` + `TopDownAtlases` +
/// `FxTuning` existing (FX-A wired it), so all are present when it runs.
#[expect(
    clippy::too_many_arguments,
    reason = "each is a distinct Bevy system param: the spawn Commands, the per-frame Time, the \
              two data tables (effect roles / atlases), the FxTuning read, the GTW-328 \
              ShotImpactResolved writer, and the two disjoint seed / playing queries — none can \
              merge without obscuring the wiring; the System fn IS the bundle"
)]
pub fn animate_impact(
    mut commands: Commands,
    time: Res<Time>,
    roles: Res<EffectRoles>,
    atlases: Res<TopDownAtlases>,
    tuning: Res<FxTuning>,
    mut impact_resolved: MessageWriter<ShotImpactResolved>,
    seeds: Query<(Entity, &super::projectile::PendingImpact)>,
    mut playing: Query<(Entity, &mut Sprite, &mut ImpactAnimation)>,
) {
    // The hot-reloadable per-frame hold each freshly-seeded impact captures.
    let frame_seconds = tuning.impact_frame_seconds;
    // Pass 1: turn each arrival seed into a playing animation (frame 0) + spawn this shot's
    // FCT pops at the impact (GTW-327), then consume the seed so it is handled exactly once.
    for (seed_entity, impact) in &seeds {
        let fx = roles.fx_for(impact.damage);
        if let Some(tile) = fx.impact.first()
            && let Some(sprite) =
                fx_sprite_scaled(*tile, Color::WHITE, impact_frame_scale(0), &atlases)
        {
            // GTW-322 — authored as a `bsn!` scene (the SAME entity tree, only the spawn SHAPE
            // changed). The atlas-indexed `Sprite` is NOT `Unpin`, and the `ImpactAnimation`
            // (a `Timer` + the frame index) has no `Default`, so BOTH ride the
            // `template(move |_| Ok(value.clone()))` closure escape hatch (the `FnTemplate`
            // output is bound by neither `Unpin` nor `Default`). The `Transform` and
            // `RenderLayers` are `Clone + Default + Unpin`, so each rides `template_value`. The
            // animation materializes on this frame's `SpawnScene` schedule; pass 2's
            // `ImpactAnimation` query picks it up next update — the same one-update-later step
            // the old `commands.spawn` produced (commands also flushed after `Update`).
            let transform = Transform::from_translation(impact.at);
            let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
            let animation = ImpactAnimation::new(impact.damage, frame_seconds);
            commands.spawn_scene((
                bsn! { template(move |_| Ok(sprite.clone())) },
                template_value(transform),
                template_value(layers),
                bsn! { template(move |_| Ok(animation.clone())) },
            ));
        }
        // GTW-327: spawn this shot's floating-combat-text pops at the impact (independent of the
        // impact SPRITE — the Text2d pops need no atlas, so they still appear when the effects
        // sheet is absent). Empty for a clean miss (no numbers).
        spawn_pops_at_anchor(&mut commands, &impact.pops, impact.anchor, &tuning);
        // GTW-328: emit the SHARED per-shot impact-resolved signal at this exact (staggered) moment
        // — carrying the shooter + the shot's verdict — so a downstream consumer (the combat-text
        // LOG; GTW-331's death-despawn next) reacts at the SAME cadence the FCT pops do, NOT all at
        // once on the fire frame. Emitted whether or not the impact sprite drew (it is independent
        // of the effects atlas, like the pops).
        impact_resolved.write(ShotImpactResolved {
            shooter: impact.shooter,
            report:  impact.report,
        });
        // Consume the seed whether or not the sheet was loaded (no re-attempt pile-up).
        commands.entity(seed_entity).despawn();
    }

    // Pass 2: step every playing impact animation, swapping tiles per frame and
    // despawning on the last frame's finish.
    let delta = time.delta();
    for (entity, mut sprite, mut anim) in &mut playing {
        match anim.advance(delta) {
            ImpactStep::Showing(frame) => {
                // Swap the sprite to this frame's impact tile (data-driven), at the frame's
                // growing scale so the burst expands into a ring. A short strip / out-of-range
                // frame leaves the prior tile up rather than panic.
                let fx = roles.fx_for(anim.damage());
                if let Some(tile) = fx.impact.get(frame)
                    && let Some(next) =
                        fx_sprite_scaled(*tile, Color::WHITE, impact_frame_scale(frame), &atlases)
                {
                    *sprite = next;
                }
            }
            ImpactStep::Finished => {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use gdtf_battle_sim::DamageType;

    use super::{IMPACT_FRAME_SCALES, ImpactAnimation, ImpactStep, impact_frame_scale};
    use crate::fx::{roles::IMPACT_FRAME_COUNT, tuning::ImpactFrameSeconds};

    /// The per-impact-frame hold the unit tests drive the stepper with — the shipped
    /// hot-reloadable default (what the resident `FxTuning` carries with no `.ron` override).
    const FRAME_SECONDS: f32 = ImpactFrameSeconds::DEFAULT;

    /// `ImpactAnimation::advance` — the exact per-frame stepper `animate_impact`'s
    /// pass-2 drives — holds each frame for its window, steps through all
    /// `IMPACT_FRAME_COUNT` frames in order, and reports `Finished` only once the
    /// LAST frame's hold elapses: the 3-FRAME-advance-then-DESPAWN contract.
    ///
    /// Driven purely off the timer (no `App` / no atlas), so it deterministically
    /// pins the lifecycle the system reads: every `Showing(n)` is the tile pass-2
    /// redraws at frame `n`, and `Finished` is the despawn signal.
    #[test]
    fn advance_steps_through_all_frames_then_finishes() {
        // A 3-frame strip is the authored impact length — assert the contract's count.
        assert_eq!(
            IMPACT_FRAME_COUNT, 3,
            "the impact animation must be a 3-frame sequence",
        );

        let mut anim = ImpactAnimation::new(DamageType::Kinetic, ImpactFrameSeconds::default());
        assert_eq!(
            anim.damage(),
            DamageType::Kinetic,
            "the animation carries the shot's damage type (selects the strip)",
        );

        // A partial tick HOLDS frame 0 (no step yet) — the same tile stays on screen.
        let half = Duration::from_secs_f32(FRAME_SECONDS / 2.0);
        assert_eq!(
            anim.advance(half),
            ImpactStep::Showing(0),
            "a partial tick holds the current frame (frame 0)",
        );

        // Each full window steps to the NEXT frame, in order, for frames 1..N-1.
        let full = Duration::from_secs_f32(FRAME_SECONDS + 0.001);
        for expected in 1..IMPACT_FRAME_COUNT {
            assert_eq!(
                anim.advance(full),
                ImpactStep::Showing(expected),
                "the {expected}-th window must step to frame {expected}",
            );
        }

        // The LAST frame's window finishing ends the animation (the despawn signal) —
        // 3 frames shown (0,1,2), then despawn.
        assert_eq!(
            anim.advance(full),
            ImpactStep::Finished,
            "the last frame's window elapsing must report Finished (despawn)",
        );
    }

    /// `impact_frame_scale` makes the impact a VISIBLY-EXPANDING shockwave (GTW-306 V3): the
    /// per-frame draw scale STRICTLY GROWS frame 0 → last (so the burst grows into a ring), is
    /// one entry per `IMPACT_FRAME_COUNT` frame, and an out-of-range frame degrades to the
    /// LARGEST (last) scale rather than snapping back to 1×. The scale is UNIFORM by
    /// construction (a single multiplier on both `custom_size` axes), which is what proves the
    /// impact is enlarged uniformly, never stretched along an axis.
    #[test]
    fn impact_frame_scale_grows_then_clamps_to_the_last_frame() {
        // One scale entry per authored impact frame.
        assert_eq!(
            IMPACT_FRAME_SCALES.len(),
            IMPACT_FRAME_COUNT,
            "the impact scale table must carry one entry per impact frame",
        );
        // The scale strictly grows frame-to-frame — the expanding-shockwave read.
        for window in IMPACT_FRAME_SCALES.windows(2) {
            if let [smaller, larger] = window {
                assert!(
                    larger > smaller,
                    "each impact frame must draw LARGER than the prior (an expanding ring): \
                     {larger} must exceed {smaller}",
                );
            }
        }
        // Every in-range frame reads its own table entry (epsilon compare — these are f32).
        for (frame, expected) in IMPACT_FRAME_SCALES.iter().enumerate() {
            assert!(
                (impact_frame_scale(frame) - *expected).abs() < f32::EPSILON,
                "frame {frame} must draw at its table scale {expected}, \
                 got {}",
                impact_frame_scale(frame),
            );
        }
        // An out-of-range frame degrades to the LAST (largest) scale, never 1× snap-back.
        let last = IMPACT_FRAME_SCALES.last().copied().unwrap_or(1.0);
        assert!(
            (impact_frame_scale(IMPACT_FRAME_COUNT + 5) - last).abs() < f32::EPSILON,
            "an out-of-range impact frame must clamp to the last (largest) scale {last}, \
             got {}",
            impact_frame_scale(IMPACT_FRAME_COUNT + 5),
        );
    }
}

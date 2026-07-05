//! The seed-consume + step-and-despawn impact system: the FCT pops, the GTW-328
//! signal emit, and the seed despawn.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{message::MessageWriter, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};

use super::{
    super::{
        projectile::spawn_pops_at_anchor, readers::fx_sprite_scaled, roles::EffectRoles,
        tuning::FxTuning,
    },
    ImpactAnimation,
    animation::{ImpactStep, impact_frame_scale},
    signal::ShotImpactResolved,
};
use crate::TopDownAtlases;

/// `Update` (`PresenterSystems::Draw`): play the 3-frame damage-type impact
/// animation — AND spawn this shot's floating-combat-text pops (GTW-327) — at each
/// arrived projectile's [`PendingImpact`](super::super::projectile::PendingImpact).
///
/// Two passes over the world, both `Commands`/`Query`-only (no `&mut World`,
/// `bevy-traps.md` #7):
///
/// 1. **Seed → animation + pops.** For each freshly-arrived
///    [`PendingImpact`](super::super::projectile::PendingImpact) FX-A's
///    [`advance_projectiles`](super::super::projectile::advance_projectiles) spawned, it
///    builds the FIRST impact tile of the damage type's strip
///    ([`EffectRoles::fx_for`](super::super::roles::EffectRoles::fx_for)`(damage).impact[0]`)
///    via [`fx_sprite_scaled`](super::super::readers::fx_sprite_scaled) at the impact world
///    point, spawns it with an [`ImpactAnimation`], SPAWNS the shot's classified
///    floating-combat-text pops at their anchor (GTW-327 — so each shot's numbers appear
///    when THIS shot's staggered impact lands, fanned out by per-shot
///    [`FctStackIndex`](super::super::fct::FctStackIndex)), EMITS the shared
///    [`ShotImpactResolved`]
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
/// hot-reloadable [`FxTuning`] resource, so a live edit to `assets/core_tuning/fx.tuning.ron`
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
    seeds: Query<(Entity, &super::super::projectile::PendingImpact)>,
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
            // HitReport is non-`Copy` since GTW-438 (it carries the rolled injury), so the
            // report is cloned out of the impact-seed component into the signal.
            report:  impact.report.clone(),
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

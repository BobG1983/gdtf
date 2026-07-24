//! The [`ShotFired`] drain: per-round projectile spawn, FCT threading, and the
//! coverage fallback.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level, Position},
    resolve_coarse::ShotKind,
    shot_fired::ShotFired,
};

use super::{
    super::{
        fct::{
            ClassifiedPop, FctSlotAllocator, FctStackIndex, anchor_cell, classify_report,
            spawn_floating_text,
        },
        readers::fx_sprite_scaled,
        roles::{EffectRoles, nearest_direction_index},
        tuning::FxTuning,
    },
    travel::{ProjectileTravel, ShotProjectile},
};
use crate::{
    GangerSprite, GangerSprites, TopDownAtlases, cell_to_world, playback::Played, sim_pos_to_world,
};

/// `Update` (`PresenterSystems::Overlay`): spawn the traveling DIRECTIONAL projectile per
/// [`ShotFired`] round.
///
/// Drains [`MessageReader<ShotFired>`](gdtf_battle_sim::shot_fired::ShotFired); for each round it picks the
/// per-damage-type FX row ([`EffectRoles::fx_for`]) and, within it, the directional tile for
/// the shot's heading ([`nearest_direction_index`] of `msg.trajectory`), then spawns ONE
/// small projectile sprite at the muzzle world point ([`sim_pos_to_world`](crate::sim_pos_to_world)
/// of `msg.muzzle`) carrying a [`ProjectileTravel`] toward the TARGET world point.
///
/// The target point depends on WHAT the round struck ([`ShotFired::kind`]): for a
/// [`Ganger`](gdtf_battle_sim::resolve_coarse::ShotKind::Ganger) hit, the bolt flies to that hit entity's
/// CURRENT rendered world position — looked up by mapping its sim [`Entity`] through
/// [`GangerSprites`](crate::GangerSprites) to its presenter sprite, then reading that sprite's
/// [`Transform`]. That rendered position ALREADY reflects the target's stance / silhouette
/// height (a prone / kneeling ganger draws lower), so the bolt angles correctly toward it with
/// NO sim change and NO 3D impact field. For any other kind (cover / slab / ground hit, or a
/// clean miss) — or if the hit ganger has no rendered sprite (off the active level) — the bolt
/// flies to the impact cell ([`cell_to_world`](crate::cell_to_world) of `msg.impact_cell` /
/// `msg.impact_level`).
///
/// The sprite is drawn at a UNIFORM
/// [`ProjectileDrawScale`](super::super::tuning::ProjectileDrawScale) of `CELL_PX` — READ from
/// the
/// hot-reloadable [`FxTuning`] resource (legibly larger so the directional comet reads, the
/// SAME factor on both axes — NEVER stretched along the vector);
/// [`advance_projectiles`](super::advance::advance_projectiles) only
/// TRANSLATES it. A missing effects sheet skips the spawn fail-closed (`fx_sprite_scaled`
/// returns [`None`]).
///
/// Since GTW-727 C33 this drains [`Played<ShotFired>`](crate::playback::Played), not the raw sim
/// [`ShotFired`] buffer: the playback cursor is the ONE writer of a `Played<ShotFired>`, and it
/// releases at most one round per frame. So the per-volley launch STAGGER that used to live here
/// (`index × InterShotSeconds`) is GONE — every bolt drained this call launches at once (delay
/// [`Duration::ZERO`](std::time::Duration::ZERO)), because the cursor already paces the volley by
/// releasing one round per frame, each earning its own dwell. Each round still spawns
/// [`Visibility::Hidden`] at the muzzle; [`advance_projectiles`](super::advance::advance_projectiles)
/// reveals it immediately (the zero launch delay has already elapsed).
///
/// The draw scale and the constant flight
/// [`ProjectileVelocity`](super::super::tuning::ProjectileVelocity) each [`ProjectileTravel`]
/// captures are read here from the resident [`FxTuning`] resource (the migrated-from-`const`,
/// hot-reloadable `.ron` table), so a live edit to `assets/core_tuning/fx.tuning.ron` re-tunes the
/// next shot's size / speed without a rebuild.
///
/// GTW-327 (slice 2): each round's classified floating-combat-text pops are computed HERE
/// (`classify_report` of its [`HitReport`](gdtf_battle_sim::resolve_and_apply::HitReport)) + their anchor cell
/// (`anchor_cell`, the hit ganger's [`Position`](gdtf_battle_sim::ganger::Position)) and threaded INTO
/// the [`ProjectileTravel`], so each shot's numbers ride its own STAGGERED flight and appear
/// when THAT shot's impact lands ([`animate_impact`](super::super::impact::animate_impact)) —
/// not all
/// at once on this drain frame. The coverage guarantee: every round that classifies to ≥ 1 pop
/// (a connecting shot) spawns a projectile that always arrives + spawns a
/// [`PendingImpact`](super::pending::PendingImpact)
/// carrying those pops; the ONE path that drops a projectile (a missing effects sheet —
/// `fx_sprite_scaled` returns [`None`]) FALLS BACK to spawning that shot's pops IMMEDIATELY
/// ([`spawn_floating_text`]) so no connecting shot's FCT is ever lost (the [`Text2d`] pops need
/// no effects atlas, only the projectile sprite does). A clean miss classifies to no pops, so it
/// rides an empty-pop bolt (still a tracer, no numbers).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`], [`Res<EffectRoles>`],
/// [`Res<FxTuning>`], [`Res<GangerSprites>`] + the read-only ganger
/// `Query<&Transform, With<GangerSprite>>` (for the hit-entity aim lookup), the read-only
/// `Query<&Position>` (GTW-327, for the pop anchor cell), the [`FctSlotAllocator`] (GTW-794, for
/// the coverage-fallback pops' stacking base), and [`MessageReader<ShotFired>`].
///
/// ORDERING (`bevy-traps.md` #3): because it consumes the [`FctSlotAllocator`] (in the coverage
/// fallback) it is registered `.after(animate_floating_text)` — the allocator must count pops
/// AFTER this frame's despawns have flushed (GTW-794 / [`FctSlotAllocator`]).
#[expect(
    clippy::too_many_arguments,
    reason = "each is a distinct Bevy system param: the spawn Commands, the three data tables \
              (atlases / effect roles / fx tuning), the two aim lookups (ganger sprite map + \
              its transforms), the GTW-327 anchor Query<&Position>, the GTW-794 FctSlotAllocator \
              (the fallback pops' stacking base), and the ShotFired reader — none can be merged \
              without obscuring the wiring; the System fn IS the bundle"
)]
pub fn spawn_shot_projectiles(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    tuning: Res<FxTuning>,
    ganger_sprites: Res<GangerSprites>,
    ganger_transforms: Query<&Transform, With<GangerSprite>>,
    positions: Query<&Position>,
    allocator: FctSlotAllocator,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    // The hot-reloadable tuning the whole volley reads (captured per spawn so a later edit
    // re-tunes the NEXT shot, not in-flight ones).
    let draw_scale = *tuning.projectile_draw_scale;
    let velocity = tuning.projectile_velocity;
    // GTW-727 C33: the per-volley launch STAGGER is gone from here. It used to compute
    // `index × inter_shot` so successive rounds of one volley left the muzzle a beat apart —
    // but the act log now carries one entry per ROUND and the playback cursor paces each with
    // its own dwell, so keeping this would be a second pacing mechanism running underneath the
    // first. The `InterShotSeconds` tuning field survives as the documented origin of
    // `RoundSeconds::DEFAULT`, so `fx.tuning.ron` needs no change and the burst's feel is
    // unchanged — the beat simply comes from the cursor now. Every bolt drained here launches
    // at once, because the cursor only ever plays ONE round per frame.
    for msg in shots.read() {
        let muzzle_world = sim_pos_to_world(msg.muzzle);
        // Where the bolt flies: a ganger hit aims at the hit entity's CURRENT rendered
        // position (its Transform already encodes its stance/silhouette height), so a
        // prone/kneeling target is angled DOWN at with no sim change. Everything else
        // (cover/slab/ground hit, or a miss) flies to the impact cell.
        let target_world = ganger_hit_world(msg.kind, &ganger_sprites, &ganger_transforms)
            .unwrap_or_else(|| cell_to_world(msg.impact_cell, msg.impact_level));

        // GTW-327: classify this shot's FCT pops + their anchor cell HERE (at the shot), to
        // thread through the staggered flight so the numbers land with this bolt's impact.
        let pops = classify_report(msg.report.as_ref());
        let anchor = anchor_cell(msg, &positions);

        // The per-damage-type row -> its 8-way rose -> the tile for this heading.
        let fx = roles.fx_for(msg.damage);
        let dir_index = nearest_direction_index(msg.trajectory.vec());
        let tile = fx.directions.get(dir_index);
        let sprite = tile.and_then(|t| fx_sprite_scaled(*t, Color::WHITE, draw_scale, &atlases));
        let Some(sprite) = sprite else {
            // No projectile sprite (no effects sheet, or a short strip) — there is no bolt to
            // carry the pops to an impact, so spawn this shot's FCT pops IMMEDIATELY rather
            // than silently dropping them (the Text2d pops need no effects atlas). Coverage
            // fallback: a connecting shot ALWAYS gets its numbers.
            spawn_pops_at_anchor(&mut commands, &allocator, &pops, anchor, &tuning);
            continue;
        };
        // No launch delay: the cursor released exactly one round this frame, so this bolt
        // leaves the muzzle now (GTW-727 C33).
        let launch_delay = std::time::Duration::ZERO;
        // GTW-322 — authored as a `bsn!` scene (the SAME entity tree, only the spawn SHAPE
        // changed). The atlas-indexed `Sprite` is NOT `Unpin` (its `Option<Handle>` fields),
        // and the `ProjectileTravel` flight (a `Timer` + the owned `Vec<ClassifiedPop>`) has no
        // `Default`, so BOTH ride the `template(move |_| Ok(value.clone()))` closure escape
        // hatch (the `FnTemplate` output is bound by neither `Unpin` nor `Default`). The
        // `Transform`, `Visibility::Hidden` (held at the muzzle until the launch delay elapses,
        // so a staggered volley reads shot-by-shot), and `RenderLayers` are all
        // `Clone + Default + Unpin`, so each rides `template_value`. The value-free
        // `ShotProjectile` marker has no `Default` — it is `.insert`ed onto the
        // synchronously-reserved id after the scene; the scene's components materialize on that
        // frame's `SpawnScene` schedule (between `Update` and `PostUpdate`).
        let travel = ProjectileTravel::new(
            muzzle_world,
            target_world,
            msg.damage,
            velocity,
            launch_delay,
            pops,
            anchor,
            msg.shooter,
            // HitReport is non-`Copy` since GTW-438 (it carries the rolled injury) — clone
            // it off the `&ShotFired` into the owned flight component.
            msg.report.clone(),
        );
        let transform = Transform::from_translation(muzzle_world);
        let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
        commands
            .spawn_scene((
                bsn! { template(move |_| Ok(sprite.clone())) },
                template_value(transform),
                template_value(Visibility::Hidden),
                template_value(layers),
                bsn! { template(move |_| Ok(travel.clone())) },
            ))
            .insert(ShotProjectile);
    }
}

/// Spawn one shot's classified floating-combat-text `pops` over `anchor` IMMEDIATELY, each at
/// the next vertical stack slot so multiple pops of the one shot fan out — SEEDED above whatever
/// pops are already live on the cell (GTW-794).
///
/// The GTW-327 COVERAGE FALLBACK (and the shared spawn used at the impact, see
/// [`animate_impact`](super::super::impact::animate_impact)): when a shot has no projectile to
/// thread
/// its pops through (a missing effects sheet — the [`Text2d`] pops still need no atlas), this
/// spawns them right away so a connecting shot never loses its numbers.
///
/// GTW-794: the shot's per-pop fan-out BASE is claimed once from the shared lifetime-aware
/// [`FctSlotAllocator`] — the slot ABOVE every pop still ALIVE on the cell (a consequence pop, a
/// fall pop, a prior shot's numbers) — so this shot's numbers stack above them instead of
/// reclaiming slot `0`. The shot's OWN pops then still ascend internally from that base
/// (`base, base + 1, base + 2, …`) so the HP number / wound tag / penetration / DOWN of one shot
/// fan out rather than overlap. `ttl` / `rise` come from the resident hot-reloadable [`FxTuning`].
pub(in crate::actors::fx) fn spawn_pops_at_anchor(
    commands: &mut Commands,
    allocator: &FctSlotAllocator,
    pops: &[ClassifiedPop],
    anchor: (Cell, Level),
    tuning: &FxTuning,
) {
    let (cell, level) = anchor;
    // GTW-794: the slot above every pop still ALIVE on this cell — this shot's numbers start
    // there, then ascend internally from it. Claimed ONCE per shot before the fan-out loop.
    let base = *allocator.next_slot(CellLevel::new(cell, level));
    for (offset, pop) in pops.iter().enumerate() {
        spawn_floating_text(
            commands,
            pop.text().clone(),
            pop.color(),
            pop.emphasis(),
            cell,
            level,
            FctStackIndex::new(base + offset),
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
    }
}

/// The CURRENT rendered world position of the ganger a round struck, or [`None`] when the
/// round did not hit a ganger (or that ganger has no rendered sprite).
///
/// For a [`ShotKind::Ganger`] outcome it maps the hit sim [`Entity`] through
/// [`GangerSprites`] to its presenter sprite [`Entity`], then reads that sprite's
/// [`Transform`] translation — the position the target is DRAWN at, which already encodes its
/// stance / silhouette height (a prone / kneeling target draws lower). Any other
/// [`ShotKind`] (cover / slab / ground / miss) returns [`None`], and so does a ganger hit
/// whose sprite is not currently mapped / rendered (e.g. on another storey) — the caller then
/// falls back to the impact cell. A pure read-only lookup: no sim change, no 3D impact field.
fn ganger_hit_world(
    kind: ShotKind,
    ganger_sprites: &GangerSprites,
    ganger_transforms: &Query<&Transform, With<GangerSprite>>,
) -> Option<Vec3> {
    let ShotKind::Ganger(sim_entity) = kind else {
        return None;
    };
    let sprite_entity = ganger_sprites.sprite_for(sim_entity)?;
    let transform = ganger_transforms.get(sprite_entity).ok()?;
    Some(transform.translation)
}

//! The GTW-306 traveling DIRECTIONAL projectile FX — the muzzle→target flight that
//! REPLACES GTW-290's smeared stretched-sprite "tracer".
//!
//! On each [`ShotFired`](gdtf_battle_sim::ShotFired) round the sim emits,
//! [`spawn_shot_projectiles`] spawns ONE small (one-tile) projectile sprite at the
//! muzzle world point, showing the per-damage-type directional tile picked for the
//! shot's heading (the [`EffectRoles`] color row → its 8-way rose →
//! [`nearest_direction_index`](super::roles::nearest_direction_index) of the
//! trajectory).
//!
//! The bolt flies at a CONSTANT VELOCITY (the hot-reloadable
//! [`ProjectileVelocity`](super::tuning::ProjectileVelocity), px/sec) — not over a
//! fixed-seconds window — so every shot shares one visual SPEED regardless of how far
//! it travels; a long shot simply spends more frames in flight. [`advance_projectiles`]
//! steps the sprite `velocity × delta` along the muzzle→target ray each frame, the
//! sprite TRAVELS at constant scale, NEVER stretched/scaled along the vector (that smear
//! was the GTW-290 bug). A straight shot is one direction, so it shows one tile for the
//! whole flight, and DESPAWNS the instant it reaches the target point.
//!
//! Where the bolt flies depends on WHAT the round struck ([`ShotFired::kind`]): for a
//! [`Ganger`](gdtf_battle_sim::ShotKind::Ganger) hit it flies to that hit entity's
//! CURRENT rendered world position (its presenter [`Transform`], looked up through
//! [`GangerSprites`](crate::GangerSprites)) — which already reflects the target's stance
//! / silhouette height, so the bolt angles correctly toward a prone / kneeling target
//! with NO sim change and NO 3D impact field. For a non-ganger hit or a clean miss it
//! flies to the impact `(cell, level)` ([`cell_to_world`](crate::cell_to_world) of
//! `msg.impact_cell` / `msg.impact_level`). When the bolt REACHES that target point it
//! despawns and spawns the impact there.
//!
//! On arrival, [`advance_projectiles`] spawns a [`PendingImpact`] at the arrival
//! point carrying the shot's [`DamageType`](gdtf_battle_sim::DamageType) — the SEAM
//! FX-B's [`animate_impact`](super::impact::animate_impact) reads to play the
//! damage type's 3-frame impact animation there. FX-A only HANDS OFF the impact
//! position + type; FX-B owns the animation.
//!
//! A burst / full-auto shot emits one [`ShotFired`] PER ROUND in one frame.
//! [`spawn_shot_projectiles`] drains them in read order and gives each round a
//! staggered LAUNCH DELAY (its read-order index × the hot-reloadable
//! [`InterShotSeconds`](super::tuning::InterShotSeconds)) so the volley animates
//! SHOT-BY-SHOT rather than all bolts leaving the muzzle at once: a
//! projectile is held INVISIBLE at the muzzle until its launch delay elapses, then
//! runs its velocity travel. So a burst reads as distinct sequential rounds (GTW-308).
//!
//! Pure VIEW (ADR-0001): it only READS [`ShotFired`] + the rendered ganger
//! [`Transform`]s + draws sprites; it adds no sim plumbing and never writes the sim.
//! Param-only throughout (`bevy-traps.md` #7).

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{Cell, DamageType, HitReport, Level, Position, ShotFired, ShotKind};

use super::{
    fct::{ClassifiedPop, FctStackIndex, anchor_cell, classify_report, spawn_floating_text},
    readers::fx_sprite_scaled,
    roles::{EffectRoles, nearest_direction_index},
    tuning::{FxTuning, ProjectileVelocity},
};
use crate::{GangerSprite, GangerSprites, TopDownAtlases, cell_to_world, sim_pos_to_world};

/// Marker tagging every traveling projectile sprite this slice spawns.
///
/// A value-free marker (the no-bare-types marker carve-out) so [`advance_projectiles`]
/// finds exactly the GTW-306 projectiles — never the FX flashes ([`FxFlash`](super::FxFlash)),
/// never terrain / ganger sprites / the camera.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShotProjectile;

/// One projectile's flight — its CONSTANT-VELOCITY travel from the muzzle world point
/// toward the target world point, after a staggered launch delay.
///
/// A NAMED grouping component (not a bare tuple): [`from`](ProjectileTravel::from) /
/// [`to`](ProjectileTravel::to) are the endpoints (world-space, the
/// [`Vec3`](bevy::math::Vec3) inner is the only carve-out the no-bare-types rule
/// allows — framework geometry the [`Transform`] consumes directly), `damage` is the
/// shot's [`DamageType`] (carried so the arrival [`PendingImpact`] knows which
/// 3-frame impact to play), `launch` is the one-shot LAUNCH-DELAY clock that holds the
/// round at the muzzle (invisible) until the volley's stagger step elapses,
/// `velocity` is the bolt's constant flight speed (the hot-reloadable
/// [`ProjectileVelocity`] CAPTURED from [`FxTuning`] at spawn, so a live `.ron` edit
/// re-tunes the next shot), and `traveled` is the distance (world px) the bolt has
/// covered along the `from → to` ray so far — advanced by `*velocity × delta` each
/// frame once launched. The launch timer + traveled distance mutate ONLY through
/// [`advance`](ProjectileTravel::advance) (no `DerefMut`).
///
/// GTW-327 (slice 2): the bolt ALSO carries this round's classified floating-combat-text
/// `pops` (the [`ClassifiedPop`] list [`classify_report`] built from the shot's
/// [`HitReport`](gdtf_battle_sim::HitReport) — empty for a clean miss) + the `anchor`
/// `(cell, level)` the pops sit on ([`anchor_cell`] of the hit ganger at the SHOT, not the
/// impact cell), so each shot's numbers ride its own staggered flight and appear when THAT
/// shot's impact lands — handed to the arrival [`PendingImpact`].
#[derive(Component, Debug, Clone)]
pub struct ProjectileTravel {
    /// The muzzle world point the flight starts at (`traveled = 0`).
    from:     Vec3,
    /// The target world point the flight ends at (`traveled = distance(from, to)`).
    to:       Vec3,
    /// The shot's damage type — handed to the arrival [`PendingImpact`] so FX-B
    /// plays the matching 3-frame impact animation.
    damage:   DamageType,
    /// The one-shot LAUNCH-DELAY clock (this round's read-order index × the
    /// hot-reloadable [`InterShotSeconds`](super::tuning::InterShotSeconds)). Until it
    /// finishes the round is held INVISIBLE at the muzzle so a multi-round volley
    /// animates shot-by-shot; once it finishes the bolt starts accumulating `traveled`.
    launch:   Timer,
    /// The bolt's constant flight speed (world px/sec) — the hot-reloadable
    /// [`ProjectileVelocity`] this round CAPTURED from [`FxTuning`] at spawn, so a live
    /// `.ron` edit re-tunes subsequently fired shots without a rebuild.
    velocity: ProjectileVelocity,
    /// Distance (world px) flown along the `from → to` ray so far — grows by
    /// `*velocity × delta` each launched frame; arrival is when it reaches the full
    /// `from → to` distance.
    traveled: f32,
    /// This shot's classified floating-combat-text pops (GTW-327) — handed to the arrival
    /// [`PendingImpact`] so [`animate_impact`](super::impact::animate_impact) spawns them
    /// when THIS shot's impact lands (staggered with the bolt). Empty for a clean miss.
    pops:     Vec<ClassifiedPop>,
    /// The `(cell, level)` this shot's pops anchor over (GTW-327) — captured at the SHOT
    /// ([`anchor_cell`], the hit ganger's cell, not the impact cell) and threaded through so
    /// the pops sit on the body that was hit.
    anchor:   (Cell, Level),
    /// The firing entity (GTW-328) — threaded through so the arrival
    /// [`PendingImpact`] can name the shooter when [`animate_impact`](super::impact::animate_impact)
    /// emits the shared [`ShotImpactResolved`](super::impact::ShotImpactResolved) signal the
    /// combat log keys its outcome lines off (the [`Entity`] is framework plumbing, the
    /// no-bare-types carve-out). It rides the bolt so the signal fires at THIS shot's staggered
    /// impact, not on the fire frame.
    shooter:  Entity,
    /// This shot's already-computed hit report (GTW-328) — the sim's verdict, threaded through to
    /// the arrival [`PendingImpact`] so the impact-resolved signal carries the data the combat log
    /// classifies a shot outcome from. [`None`] for a geometry-only round (read as a miss).
    report:   Option<HitReport>,
}

impl ProjectileTravel {
    /// Start a fresh projectile flight from `from` toward `to` carrying `damage`, flying at
    /// `velocity`, after a `launch_delay` hold at the muzzle, carrying this shot's classified
    /// floating-combat-text `pops` anchored over `anchor`.
    ///
    /// `velocity` is the hot-reloadable [`ProjectileVelocity`] the caller READ from the
    /// resident [`FxTuning`] resource — CAPTURED here so a later `.ron` edit re-tunes the
    /// NEXT shot rather than mid-flight ones. `launch_delay` is this round's stagger offset
    /// (its read-order index × the tuning's
    /// [`InterShotSeconds`](super::tuning::InterShotSeconds)); a
    /// [`Duration::ZERO`](std::time::Duration::ZERO) delay launches at once (round 0 / a
    /// single shot — a zero-duration `Once` timer is already finished). The launch hold is a
    /// [`TimerMode::Once`] clock; once it finishes the bolt flies at `*velocity` (px/sec)
    /// until it has covered the whole `from → to` distance, at which point it has arrived and
    /// is despawned.
    ///
    /// `pops` (GTW-327) is this round's [`ClassifiedPop`] list ([`classify_report`] of the
    /// shot's report — empty for a clean miss) and `anchor` the `(cell, level)` they sit on
    /// ([`anchor_cell`]); they are carried through to the arrival [`PendingImpact`] so the
    /// numbers appear when THIS shot's impact lands rather than on the drain frame.
    ///
    /// `pub(in crate::actors::fx)`: it takes the FX-internal [`ClassifiedPop`], so it stays sealed to
    /// the FX layer (the `spawn_shot_projectiles` caller + the in-crate flight tests construct
    /// it; nothing outside `crate::fx` needs to build a flight).
    ///
    /// `shooter` + `report` (GTW-328) ride through to the arrival [`PendingImpact`] so
    /// [`animate_impact`](super::impact::animate_impact) can emit the shared
    /// [`ShotImpactResolved`](super::impact::ShotImpactResolved) signal — naming the shooter and
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
    /// reached the full `from → to` distance — the signal [`advance_projectiles`] despawns +
    /// spawns the [`PendingImpact`] on. Wraps the inner [`Timer`] + distance so they mutate
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

    /// The shot's damage type — handed to the arrival [`PendingImpact`].
    #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.damage
    }

    /// This shot's classified floating-combat-text pops (GTW-327), moved out at arrival so the
    /// [`PendingImpact`] owns them (the bolt is despawned the same frame, so the [`Vec`] is not
    /// needed on it afterward). Empty for a clean miss.
    #[must_use]
    fn take_pops(&mut self) -> Vec<ClassifiedPop> {
        std::mem::take(&mut self.pops)
    }

    /// The `(cell, level)` this shot's pops anchor over (GTW-327) — handed to the arrival
    /// [`PendingImpact`].
    #[must_use]
    const fn anchor(&self) -> (Cell, Level) {
        self.anchor
    }

    /// The firing entity (GTW-328) — handed to the arrival [`PendingImpact`] so the
    /// impact-resolved signal names the shooter.
    #[must_use]
    const fn shooter(&self) -> Entity {
        self.shooter
    }

    /// This shot's hit report (GTW-328) — handed to the arrival [`PendingImpact`] so the
    /// impact-resolved signal carries the verdict the combat log classifies. Cloned (the
    /// [`HitReport`] is non-`Copy` since GTW-438 — it carries the rolled injury).
    #[must_use]
    fn report(&self) -> Option<HitReport> {
        self.report.clone()
    }
}

/// A projectile has ARRIVED — the SEAM FX-B reads to play the 3-frame impact.
///
/// [`advance_projectiles`] spawns one of these (a bare entity carrying ONLY this
/// component) at the arrival point the instant a projectile despawns;
/// [`animate_impact`](super::impact::animate_impact) queries for them and steps the
/// [`damage`](PendingImpact::damage) type's 3 impact frames there (AND, GTW-327, spawns this
/// shot's floating-combat-text pops at the [`anchor`](PendingImpact::anchor)) before despawning
/// the impact entity. FX-A defines + spawns this so FX-B's `impact.rs` only fills the animation
/// body (no `mod.rs` collision).
///
/// A NAMED grouping component: [`at`](PendingImpact::at) is the impact world point
/// (the [`Vec3`](bevy::math::Vec3) carve-out — framework geometry), `damage` the
/// shot's [`DamageType`] (so FX-B picks the matching impact strip), and — GTW-327 —
/// [`pops`](PendingImpact::pops) the shot's classified floating-combat-text pops +
/// [`anchor`](PendingImpact::anchor) the `(cell, level)` they sit on, so the numbers appear at
/// THIS shot's staggered impact. Not [`Copy`] (it owns the pop [`Vec`]); the
/// [`fields`](PendingImpact) are `pub(in crate::actors::fx)` so `animate_impact` reads + consumes them
/// while the type stays sealed to the FX layer.
#[derive(Component, Debug, Clone)]
pub struct PendingImpact {
    /// The world point the projectile arrived at — where the impact animation plays.
    pub(in crate::actors::fx) at:      Vec3,
    /// The shot's damage type — selects which 3-frame impact strip FX-B animates.
    pub(in crate::actors::fx) damage:  DamageType,
    /// This shot's classified floating-combat-text pops (GTW-327) — spawned by
    /// [`animate_impact`](super::impact::animate_impact) at the [`anchor`](PendingImpact::anchor)
    /// when the impact lands. Empty for a clean miss (no pops).
    pub(in crate::actors::fx) pops:    Vec<ClassifiedPop>,
    /// The `(cell, level)` this shot's pops anchor over (GTW-327) — the hit ganger's cell at
    /// the SHOT (not the impact cell), threaded through the staggered flight.
    pub(in crate::actors::fx) anchor:  (Cell, Level),
    /// The firing entity (GTW-328) — so [`animate_impact`](super::impact::animate_impact) names
    /// the shooter in the [`ShotImpactResolved`](super::impact::ShotImpactResolved) signal it
    /// emits when this impact resolves. The [`Entity`] is framework plumbing (the no-bare-types
    /// carve-out).
    pub(in crate::actors::fx) shooter: Entity,
    /// This shot's hit report (GTW-328) — the sim's verdict, carried into the
    /// [`ShotImpactResolved`](super::impact::ShotImpactResolved) signal so the combat log
    /// classifies the shot outcome at THIS shot's staggered impact. [`None`] reads as a miss.
    pub(in crate::actors::fx) report:  Option<HitReport>,
}

/// `Update` (`PresenterSystems::Draw`): spawn the traveling DIRECTIONAL projectile per
/// [`ShotFired`] round.
///
/// Drains [`MessageReader<ShotFired>`](gdtf_battle_sim::ShotFired); for each round it picks the
/// per-damage-type FX row ([`EffectRoles::fx_for`]) and, within it, the directional tile for
/// the shot's heading ([`nearest_direction_index`] of `msg.trajectory`), then spawns ONE
/// small projectile sprite at the muzzle world point ([`sim_pos_to_world`](crate::sim_pos_to_world)
/// of `msg.muzzle`) carrying a [`ProjectileTravel`] toward the TARGET world point.
///
/// The target point depends on WHAT the round struck ([`ShotFired::kind`]): for a
/// [`Ganger`](gdtf_battle_sim::ShotKind::Ganger) hit, the bolt flies to that hit entity's
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
/// [`ProjectileDrawScale`](super::tuning::ProjectileDrawScale) of `CELL_PX` — READ from the
/// hot-reloadable [`FxTuning`] resource (legibly larger so the directional comet reads, the
/// SAME factor on both axes — NEVER stretched along the vector); [`advance_projectiles`] only
/// TRANSLATES it. A missing effects sheet skips the spawn fail-closed (`fx_sprite_scaled`
/// returns [`None`]).
///
/// A burst / full-auto shot emits one [`ShotFired`] per round in ONE frame, so this spawns one
/// projectile per round — and STAGGERS them (GTW-308): each round's read-order index (0, 1, 2, …)
/// × the tuning's [`InterShotSeconds`](super::tuning::InterShotSeconds) is its
/// [`ProjectileTravel`] launch delay, so the volley animates shot-by-shot rather than as a
/// single fat bolt. Each round spawns [`Visibility::Hidden`] (held invisible at the muzzle);
/// [`advance_projectiles`] reveals it once its launch delay elapses. The read-order index
/// counts every round drained this call (across multiple bursts in one frame, too), which is
/// the order the rounds leave the muzzle.
///
/// The draw scale, the constant flight [`ProjectileVelocity`] each [`ProjectileTravel`]
/// captures, and the inter-shot stagger are ALL read here from the resident [`FxTuning`]
/// resource (the migrated-from-`const`, hot-reloadable `.ron` table), so a live edit to
/// `assets/tiles/fx_tuning.ron` re-tunes the next shot's size / speed / spacing without a
/// rebuild.
///
/// GTW-327 (slice 2): each round's classified floating-combat-text pops are computed HERE
/// ([`classify_report`] of its [`HitReport`](gdtf_battle_sim::HitReport)) + their anchor cell
/// ([`anchor_cell`], the hit ganger's [`Position`](gdtf_battle_sim::Position)) and threaded INTO
/// the [`ProjectileTravel`], so each shot's numbers ride its own STAGGERED flight and appear
/// when THAT shot's impact lands ([`animate_impact`](super::impact::animate_impact)) — not all
/// at once on this drain frame. The coverage guarantee: every round that classifies to ≥ 1 pop
/// (a connecting shot) spawns a projectile that always arrives + spawns a [`PendingImpact`]
/// carrying those pops; the ONE path that drops a projectile (a missing effects sheet —
/// `fx_sprite_scaled` returns [`None`]) FALLS BACK to spawning that shot's pops IMMEDIATELY
/// ([`spawn_floating_text`]) so no connecting shot's FCT is ever lost (the [`Text2d`] pops need
/// no effects atlas, only the projectile sprite does). A clean miss classifies to no pops, so it
/// rides an empty-pop bolt (still a tracer, no numbers).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`], [`Res<EffectRoles>`],
/// [`Res<FxTuning>`], [`Res<GangerSprites>`] + the read-only ganger
/// `Query<&Transform, With<GangerSprite>>` (for the hit-entity aim lookup), the read-only
/// `Query<&Position>` (GTW-327, for the pop anchor cell), and [`MessageReader<ShotFired>`].
#[expect(
    clippy::too_many_arguments,
    reason = "each is a distinct Bevy system param: the spawn Commands, the three data tables \
              (atlases / effect roles / fx tuning), the two aim lookups (ganger sprite map + \
              its transforms), the GTW-327 anchor Query<&Position>, and the ShotFired reader — \
              none can be merged without obscuring the wiring; the System fn IS the bundle"
)]
pub fn spawn_shot_projectiles(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    tuning: Res<FxTuning>,
    ganger_sprites: Res<GangerSprites>,
    ganger_transforms: Query<&Transform, With<GangerSprite>>,
    positions: Query<&Position>,
    mut shots: MessageReader<ShotFired>,
) {
    // The hot-reloadable tuning the whole volley reads (captured per spawn so a later edit
    // re-tunes the NEXT shot, not in-flight ones).
    let draw_scale = *tuning.projectile_draw_scale;
    let velocity = tuning.projectile_velocity;
    let inter_shot = *tuning.inter_shot_seconds;
    // The read-order index of each round drained this call — its position in the volley, which
    // sets its stagger offset so successive rounds leave the muzzle one step apart.
    for (round, msg) in shots.read().enumerate() {
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
            spawn_pops_at_anchor(&mut commands, &pops, anchor, &tuning);
            continue;
        };
        // This round's launch delay = its read-order index × the inter-shot step (round 0 = 0,
        // launches at once). Scaling a `Duration` by the `u32` index keeps it exact with no `as`
        // float cast (`Duration` implements `Mul<u32>`).
        let round_index = u32::try_from(round).unwrap_or(u32::MAX);
        let launch_delay = std::time::Duration::from_secs_f32(inter_shot) * round_index;
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
/// the next per-shot vertical stack slot so multiple pops of the one shot fan out.
///
/// The GTW-327 COVERAGE FALLBACK (and the shared spawn used at the impact, see
/// [`animate_impact`](super::impact::animate_impact)): when a shot has no projectile to thread
/// its pops through (a missing effects sheet — the [`Text2d`] pops still need no atlas), this
/// spawns them right away so a connecting shot never loses its numbers. The pops fan DOWN by
/// their per-shot [`FctStackIndex`] (`0, 1, 2, …`) so the HP number / wound tag / penetration /
/// DOWN of one shot stack rather than overlap. `ttl` / `rise` come from the resident
/// hot-reloadable [`FxTuning`].
pub(in crate::actors::fx) fn spawn_pops_at_anchor(
    commands: &mut Commands,
    pops: &[ClassifiedPop],
    anchor: (Cell, Level),
    tuning: &FxTuning,
) {
    let (cell, level) = anchor;
    for (slot, pop) in pops.iter().enumerate() {
        spawn_floating_text(
            commands,
            pop.text().clone(),
            pop.color(),
            pop.emphasis(),
            cell,
            level,
            FctStackIndex::new(slot),
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

/// `Update` (`PresenterSystems::Draw`): advance every traveling projectile and HAND OFF its
/// impact on arrival.
///
/// Advances each [`ProjectileTravel`] by the frame [`Res<Time>`] delta. A round whose staggered
/// launch delay has NOT yet elapsed is held INVISIBLE at the muzzle ([`Visibility::Hidden`], no
/// translation) so a multi-round volley animates shot-by-shot; the frame its launch delay
/// elapses it becomes [`Visibility::Visible`] and thereafter its `Transform` is written to its
/// current travel point ([`ProjectileTravel::position`]) — constant-VELOCITY TRANSLATION, the
/// sprite never stretched/scaled. The instant the bolt has flown the whole `from → to` distance
/// ([`ProjectileTravel::advance`] returns `true`), it `Commands::entity(e).despawn`s the
/// projectile AND `Commands::spawn`s a [`PendingImpact`] at the arrival point carrying the shot's
/// damage type — the seam FX-B's [`animate_impact`](super::impact::animate_impact) reads. It
/// touches ONLY [`ShotProjectile`]-marked entities; it needs no `BattleInProgress` gate (inert
/// with no projectiles — the query is empty — so a projectile spawned during a battle still
/// completes its flight after it ends).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the despawn / impact spawn, [`Res<Time>`]
/// for the delta, and the `(Entity, &mut Transform, &mut Visibility, &mut ProjectileTravel)`
/// query (`With<ShotProjectile>`).
pub fn advance_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<
        (
            Entity,
            &mut Transform,
            &mut Visibility,
            &mut ProjectileTravel,
        ),
        With<ShotProjectile>,
    >,
) {
    let delta = time.delta();
    for (entity, mut transform, mut visibility, mut travel) in &mut projectiles {
        let arrived = travel.advance(delta);
        if !travel.launched() {
            // Still parked at the muzzle — keep it hidden, do not move it yet.
            continue;
        }
        // Launched: reveal it (set_if_neq avoids a needless change-detection write each frame)
        // and translate to the current flight point (never scale — that was the bug).
        visibility.set_if_neq(Visibility::Visible);
        transform.translation = travel.position();
        if arrived {
            // Hand the impact off to FX-B at the arrival point, carrying this shot's classified
            // FCT pops + their anchor (GTW-327) so the numbers appear with THIS bolt's impact,
            // then despawn the bolt (its pops are MOVED into the impact, not duplicated).
            //
            // GTW-322 — the seam entity carries ONLY `PendingImpact`, which owns the pop `Vec`
            // (no `Default`), so it is spawned as a single-`bsn!` scene via the
            // `template(move |_| Ok(value.clone()))` closure escape hatch (the bare entity the
            // old `commands.spawn(PendingImpact { .. })` produced — same component, deferred to
            // that frame's `SpawnScene` schedule).
            let pending = PendingImpact {
                at:      travel.arrival(),
                damage:  travel.damage(),
                anchor:  travel.anchor(),
                shooter: travel.shooter(),
                report:  travel.report(),
                pops:    travel.take_pops(),
            };
            commands.spawn_scene(bsn! { template(move |_| Ok(pending.clone())) });
            commands.entity(entity).despawn();
        }
    }
}

//! Transient FX registration: the flash / projectile / impact readers, the grenade
//! blast reader, and the consequence-FCT palette families.

use bevy::{ecs::message::Messages, prelude::*};
use gdtf_battle_sim::{
    ArmorBroken, BattleInProgress, Bleeding, CoverDestroyed, FallOccurred, ShotFired,
    acts::{MeleeResolved, ThrowResolved},
};

use crate::{
    ArmorBrokenFct, BleedingFct, ConsequenceFctAppExt, DotFct, EffectRoles, FieldFct, FxTuning,
    InjuryFct, OnDeathFct, PresenterSystems, ShotImpactResolved, SuppressionFct, TopDownAtlases,
    advance_projectiles, animate_floating_text, animate_impact, expire_flashes, read_armor_broken,
    read_bleeding, read_cover_destroyed, read_fall_occurred, read_melee_resolved,
    read_throw_resolved, register_consequence_fct_core, spawn_shot_projectiles,
};

/// Registers the GTW-220 (S6) transient FX-flash readers + the one-shot expiry clock into the
/// already-defined [`PresenterSystems::Draw`] band.
///
/// Each reader drains a [`MessageReader`] over one sim FX message
/// ([`Bleeding`](gdtf_battle_sim::Bleeding) / [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) /
/// [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) — the consequence flashes — plus the
/// GTW-290 [`ShotFired`](gdtf_battle_sim::ShotFired) firing FX) the sim already emits, looks
/// up the cell via `Query<&Position>` / the message geometry (read-only, NO sim plumbing
/// added), and `Commands::spawn`s the short-lived effects sprite(s). The GTW-306 firing FX is
/// split across [`spawn_shot_projectiles`] + [`advance_projectiles`] (the traveling directional
/// projectile that LERPs muzzle→impact then despawns, handing off a `PendingImpact`) and
/// [`animate_impact`] (FX-B's 3-frame impact animation at the arrival point); it does NOT
/// duplicate the three consequence flashes. The standalone muzzle flash was REMOVED (GTW-307):
/// it rendered oversized at the shooter's feet and read poorly, so the traveling projectile
/// (departing the muzzle) IS the fire signal.
///
/// Each reader's gate is `resource_exists::<BattleInProgress>` AND every render resource it
/// reads — the [`EffectRoles`] data table + [`TopDownAtlases`] — AND its own `Messages<M>`
/// buffer existing. The render-resource guards make a `MinimalPlugins` headless app with no
/// [`AssetServer`] (those resources absent) simply NOT draw rather than failing param
/// validation (`bevy-traps.md` #1; the ticket's "a no-resource state must NOT panic the
/// draw"). The `Messages<M>` guard is the matching mandatory gate for the [`MessageReader<M>`]
/// param itself: a [`MessageReader<M>`] panics param validation when its `Messages<M>` buffer
/// is absent (the sim's `BattleSimPlugin` registers all three buffers during a real battle,
/// but a focused headless harness may insert `BattleInProgress` + the render tables WITHOUT a
/// given FX buffer), so each reader is independently gated on the one buffer it drains.
///
/// `expire_flashes` is the one-shot despawn clock: it needs only `Res<Time>` + the [`FxFlash`](crate::FxFlash)
/// query (no render resource, no message buffer) and is inert with no flashes (the query is
/// empty), so it is registered unguarded by `BattleInProgress` — a flash spawned during a
/// battle still expires after the battle ends.
pub(super) fn register_fx_flash_systems(app: &mut App) {
    // GTW-328: register the shared per-shot impact-resolved signal buffer `animate_impact` emits
    // (the combat log drains it). `add_message` is idempotent and creates the `Messages<T>`
    // resource so the `MessageWriter` param is always valid even when `animate_impact` is gated
    // off (`bevy-traps.md` #4 — a MessageWriter panics validation without its buffer); a downstream
    // consumer (the combat-log plugin in `gdtf_app`) also registers it idempotently.
    app.add_message::<ShotImpactResolved>();
    // GTW-507: register the sim's MeleeResolved buffer idempotently so `read_melee_resolved`'s
    // MessageReader param is valid in a presenter-only headless harness (the sim's SimActsPlugin
    // also registers it in a real battle — `add_message` is IDEMPOTENT; the CoverDestroyed
    // precedent, bevy-traps.md #4).
    app.add_message::<MeleeResolved>();
    // GTW-524: register the sim's FallOccurred buffer idempotently so `read_fall_occurred`'s
    // MessageReader param is valid in a presenter-only headless harness (the sim's FallsPlugin
    // registers it in a real battle — `add_message` is IDEMPOTENT; the MeleeResolved precedent).
    app.add_message::<FallOccurred>();
    let render_gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<EffectRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    app.add_systems(
        Update,
        read_bleeding.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<Bleeding>>),
        ),
    )
    .add_systems(
        Update,
        read_armor_broken.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<ArmorBroken>>),
        ),
    )
    .add_systems(
        Update,
        read_cover_destroyed.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<CoverDestroyed>>),
        ),
    )
    // GTW-507: the close-combat STRIKE flash. Drains the sim's MeleeResolved signal (one per
    // connecting melee hit) and spawns a one-frame strike glyph at the struck target cell,
    // data-driven via the EffectRoles `melee_strike` tile (the read_cover_destroyed precedent).
    // Gated on the SAME render resources + the MeleeResolved buffer its MessageReader drains (a
    // MessageReader param panics validation without its buffer — bevy-traps.md #1 / #4).
    .add_systems(
        Update,
        read_melee_resolved.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<Messages<MeleeResolved>>),
        ),
    )
    // GTW-524: the fall-impact flash + FCT "Fell" pop. Drains the sim's FallOccurred signal
    // (one per ganger that dropped a storey after a slab was destroyed beneath it) and spawns
    // a one-frame impact glyph at the LANDING cell, data-driven via the EffectRoles
    // `fall_impact` tile, plus a rise-and-fade "Fell" FCT pop. The tint scales with storeys
    // fallen (a structural relation). ADDITIVE to the wound/bleed/injury flashes the fall
    // damage drives (the existing signals handle those; this is ONLY the fall-event glyph).
    // Gated on the render resources + FxTuning (for the FCT pop lifetime + rise) + the
    // FallOccurred message buffer (MessageReader panics validation without its buffer —
    // bevy-traps.md #1 / #4).
    .add_systems(
        Update,
        read_fall_occurred.in_set(PresenterSystems::Draw).run_if(
            render_gate
                .clone()
                .and_then(resource_exists::<FxTuning>)
                .and_then(resource_exists::<Messages<FallOccurred>>),
        ),
    )
    // GTW-306: the firing FX. spawn_shot_projectiles spawns the traveling DIRECTIONAL
    // projectile off the ShotFired buffer (the muzzle flash was removed in GTW-307 — the
    // departing projectile IS the fire signal) under the same render gate; animate_impact
    // (below) drains the SAME buffer independently (a buffered message survives a frame).
    // GTW-327 (slice 2): spawn_shot_projectiles now ALSO classifies each shot's FCT pops
    // (classify_report) + anchor and threads them THROUGH the projectile -> PendingImpact ->
    // animate_impact pipeline, so each shot's numbers appear at its own STAGGERED impact (no
    // separate read_shot_fired_text system — the immediate-spawn that dumped a whole volley's
    // numbers on the drain frame is gone). Both READ the hot-reloadable FxTuning resource, so
    // each adds it to its gate so a pre-resolve frame (tuning not yet loaded) does not fail
    // param validation.
    .add_systems(
        Update,
        spawn_shot_projectiles
            .in_set(PresenterSystems::Draw)
            .run_if(
                render_gate
                    .clone()
                    .and_then(resource_exists::<FxTuning>)
                    .and_then(resource_exists::<Messages<ShotFired>>),
            ),
    )
    // GTW-306: the 3-frame impact animation (FX-B fills the body). Gated on the same render
    // resources it reads (EffectRoles + TopDownAtlases + BattleInProgress + the
    // hot-reloadable FxTuning) so FX-B edits only impact.rs — never this registration.
    // GTW-328: it ALSO emits the shared per-shot `ShotImpactResolved` signal at each impact
    // (the combat log keys its outcome lines off it). Its buffer is registered just below via
    // `add_message` (idempotent), so the `MessageWriter` param is always valid (`bevy-traps.md`
    // #4) — no extra run gate is needed for the writer (a writer needs only the buffer, which
    // the registration guarantees).
    .add_systems(
        Update,
        animate_impact
            .in_set(PresenterSystems::Draw)
            .run_if(render_gate.and_then(resource_exists::<FxTuning>)),
    )
    // GTW-306: advance every traveling projectile + hand off its impact. Needs only Time + the
    // ShotProjectile query (no render resource / message buffer), inert with none — registered
    // unguarded like expire_flashes so an in-flight projectile completes after a battle ends.
    .add_systems(Update, advance_projectiles.in_set(PresenterSystems::Draw))
    // GTW-302 (slice 2): rise + fade + despawn every live floating-combat-text pop. Like
    // expire_flashes / advance_projectiles it needs only Time + its own (FloatingCombatText)
    // query — no render resource, no message buffer — and is inert with no pops, so it is
    // registered unguarded by BattleInProgress: a pop spawned during a battle still completes
    // its rise/fade after the battle ends. The reader slices (3-4) SPAWN the pops; this is the
    // generic animator the primitive owns.
    .add_systems(Update, animate_floating_text.in_set(PresenterSystems::Draw))
    .add_systems(Update, expire_flashes.in_set(PresenterSystems::Draw));
}

/// Register the GTW-546 grenade-BLAST FX reader on the `PresenterSystems::Draw` band —
/// extracted from [`register_fx_flash_systems`] to keep that fn under the `too_many_lines` lint
/// (the `register_on_death_systems` precedent).
///
/// [`read_throw_resolved`] drains the sim's [`ThrowResolved`](gdtf_battle_sim::acts::ThrowResolved)
/// signal (one per resolved throw) and SEEDS a [`PendingImpact`](crate::PendingImpact) at the
/// arc's LANDING cell so the EXISTING [`animate_impact`] plays the grenade's damage-type
/// expanding-shockwave impact strip there — the SAME `AoE` hit FX the fire path renders, reused
/// (the throw's sim blast fold emits no [`ShotFired`], so it would otherwise be an invisible HP
/// drain). The seed is spawned via `spawn_scene`, so it materializes on the frame's `SpawnScene`
/// schedule and [`animate_impact`] picks it up the NEXT update — the SAME one-update handoff a
/// shot's arrived projectile uses ([`advance_projectiles`] also seeds its `PendingImpact` via
/// `spawn_scene`), so no explicit ordering is needed.
///
/// Registers the sim's [`ThrowResolved`](gdtf_battle_sim::acts::ThrowResolved) buffer idempotently
/// (`add_message` is IDEMPOTENT; the `MeleeResolved` precedent — the sim's `SimActsPlugin`
/// registers it in a real battle, this presenter-only harness adds it so the `MessageReader` param
/// validates, `bevy-traps.md` #4). Gated on the SAME render resources + the `ThrowResolved` buffer
/// its `MessageReader` drains.
pub(super) fn register_throw_blast_systems(app: &mut App) {
    app.add_message::<ThrowResolved>();
    let render_gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<EffectRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    app.add_systems(
        Update,
        read_throw_resolved
            .in_set(PresenterSystems::Draw)
            .run_if(render_gate.and_then(resource_exists::<Messages<ThrowResolved>>)),
    );
}

/// Registers the GTW-572 CONSEQUENCE-FCT PALETTE: the shared core (the per-frame
/// [`FctStackCounter`](crate::FctStackCounter) + its reset, explicitly ordered before the
/// reader set — `bevy-traps.md` #3) and then ONE registrar line per consequence family.
///
/// This replaced the per-family reader walls (the old `read_consequence_fct` /
/// `read_injury_fct` / `read_suppression_fct` / `read_dot_fct` / `read_field_fct` /
/// `read_on_death_fct` registrations): each family is now a
/// [`ConsequenceFct`](crate::ConsequenceFct) impl in its own file under `fct/families/`,
/// driven by the ONE generic [`read_consequence_fct`](crate::read_consequence_fct) reader.
/// Adding a consequence = one family file + the one
/// [`add_consequence_fct`](ConsequenceFctAppExt::add_consequence_fct) line below (see the
/// families module doc for the whole recipe).
///
/// The registrar NEVER calls `add_message` (GTW-572 C4): the two old presenter-side
/// idempotent registrations (`FieldTicked`, `OnDeathOccurred`) are GONE — in a live battle
/// the sim's plugins register every buffer, and a presenter-only headless harness that
/// omits a family's `Messages<M>` buffer simply keeps that family's reader INERT (the
/// `run_if(resource_exists::<Messages<M>>)` gate; `bevy-traps.md` #1 / #4).
pub(super) fn register_consequence_fct_families(app: &mut App) {
    register_consequence_fct_core(app);
    app.add_consequence_fct::<BleedingFct>()
        .add_consequence_fct::<ArmorBrokenFct>()
        .add_consequence_fct::<InjuryFct>()
        .add_consequence_fct::<SuppressionFct>()
        .add_consequence_fct::<DotFct>()
        .add_consequence_fct::<FieldFct>()
        .add_consequence_fct::<OnDeathFct>();
}

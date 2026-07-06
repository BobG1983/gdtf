//! Transient FX registration: the flash / projectile / impact readers, the grenade
//! blast reader, and the consequence-FCT palette families.

use bevy::{ecs::message::Messages, prelude::*};
use gdtf_battle_sim::{
    acts::{MeleeResolved, ThrowResolved},
    armor_wear::ArmorBroken,
    effects::bleed::Bleeding,
    falls::FallOccurred,
    occupancy_sync::CoverDestroyed,
    prelude::BattleInProgress,
    shot_fired::ShotFired,
};

use crate::{
    ArmorBrokenFct, BleedingFct, ConsequenceFctAppExt, DotFct, EffectRoles, FieldFct,
    FxReaderAppExt, FxTuning, InjuryFct, OnDeathFct, PresenterSystems, ShotImpactResolved,
    SuppressionFct, TopDownAtlases, advance_projectiles, animate_floating_text, animate_impact,
    expire_flashes, read_armor_broken, read_bleeding, read_cover_destroyed, read_fall_occurred,
    read_melee_resolved, read_throw_resolved, register_consequence_fct_core,
    spawn_shot_projectiles,
};

/// Registers the GTW-220 (S6) transient FX-flash readers, the GTW-546 grenade BLAST
/// reader, the GTW-306 firing FX, and the one-shot expiry clock into the
/// [`PresenterSystems::Overlay`] stage (GTW-623 — ordering by STAGE MEMBERSHIP: the
/// transient FX draw over the composed scene, never a pairwise `.after` edge).
///
/// # The six flash-family readers — one registrar line each (GTW-623 C3)
///
/// Each reader drains a [`MessageReader`] over one sim FX message
/// ([`Bleeding`](gdtf_battle_sim::effects::bleed::Bleeding) / [`ArmorBroken`](gdtf_battle_sim::armor_wear::ArmorBroken) /
/// [`CoverDestroyed`](gdtf_battle_sim::occupancy_sync::CoverDestroyed) /
/// [`MeleeResolved`](gdtf_battle_sim::acts::MeleeResolved) (GTW-507) /
/// [`FallOccurred`](gdtf_battle_sim::falls::FallOccurred) (GTW-524) /
/// [`ThrowResolved`](gdtf_battle_sim::acts::ThrowResolved) (GTW-546)), looks up the cell
/// via `Query<&Position>` / the message geometry (read-only, NO sim plumbing added), and
/// `Commands::spawn`s the short-lived effects sprite(s). They register through the ONE
/// generic [`FxReaderAppExt::add_fx_reader`] registrar, which carries the whole gate
/// shape — `BattleInProgress` + [`EffectRoles`] + [`TopDownAtlases`] + a REAL
/// `Messages<M>` gate — and NEVER calls `add_message` (GTW-623 C4, the GTW-572 C4
/// convention): the sim's plugins register every sim-owned buffer in a live battle, and
/// a presenter-only headless harness that omits a buffer keeps that reader INERT
/// instead of failing param validation (`bevy-traps.md` #1 / #4). The old presenter-side
/// idempotent `add_message` calls for `MeleeResolved` / `FallOccurred` /
/// `ThrowResolved` are GONE — the focused fx harnesses seed the buffers they write
/// themselves (the `FieldTicked` / `OnDeathOccurred` precedent).
///
/// [`read_fall_occurred`] composes ONE extra gate onto the shared shape — the
/// hot-reloadable [`FxTuning`], read for its `"Fell"` FCT pop's lifetime + rise — by
/// passing the pre-gated system into the registrar (run conditions AND together).
/// [`read_throw_resolved`] SEEDS a [`PendingImpact`](crate::PendingImpact) at the throw
/// arc's landing cell so the EXISTING [`animate_impact`] plays the grenade's
/// expanding-shockwave strip there (the throw's sim blast fold emits no `ShotFired`, so
/// it would otherwise be an invisible HP drain); the seed materializes on the frame's
/// `SpawnScene` schedule and `animate_impact` picks it up the NEXT update — the same
/// one-update handoff an arrived projectile uses, so no explicit ordering is needed.
///
/// # The firing FX + animators (direct registrations)
///
/// The GTW-306 firing FX is split across [`spawn_shot_projectiles`] +
/// [`advance_projectiles`] (the traveling directional projectile that LERPs
/// muzzle→impact then despawns, handing off a `PendingImpact`) and [`animate_impact`]
/// (FX-B's 3-frame impact animation at the arrival point, which ALSO emits the shared
/// per-shot [`ShotImpactResolved`] signal — GTW-328). They stay directly registered:
/// `spawn_shot_projectiles` adds `FxTuning` to the reader gate shape, and
/// `animate_impact` drains NO message buffer (it consumes `PendingImpact` entities), so
/// neither fits the registrar's `Messages<M>`-gated shape exactly. The standalone muzzle
/// flash was REMOVED (GTW-307): the traveling projectile (departing the muzzle) IS the
/// fire signal.
///
/// [`expire_flashes`] / [`advance_projectiles`] / [`animate_floating_text`] are the
/// unguarded animators: each needs only `Res<Time>` + its own query (no render
/// resource, no message buffer) and is inert with nothing live, so none is gated on
/// `BattleInProgress` — a flash / bolt / pop spawned during a battle still completes
/// after the battle ends.
pub(super) fn register_fx_flash_systems(app: &mut App) {
    // GTW-328: register the shared per-shot impact-resolved signal buffer `animate_impact`
    // emits (the combat log drains it). This is a PRESENTER-OWNED message (defined in
    // `actors/fx/impact/`), so registering it here is the owner registering its own buffer —
    // NOT a sim-owned-buffer registration (GTW-623 C4 leaves it untouched). `add_message` is
    // idempotent and creates the `Messages<T>` resource so the `MessageWriter` param is always
    // valid even when `animate_impact` is gated off (`bevy-traps.md` #4); the downstream
    // consumer (the GTW-620 combat-log forwarder in `combat_log.rs`) gates on this buffer
    // existing rather than registering it.
    app.add_message::<ShotImpactResolved>();
    // GTW-623 C3: the six flash-family readers — one registrar line each. The registrar
    // carries the stage + the shared render gate + the per-message `Messages<M>` gate.
    app.add_fx_reader::<Bleeding, _>(read_bleeding)
        .add_fx_reader::<ArmorBroken, _>(read_armor_broken)
        .add_fx_reader::<CoverDestroyed, _>(read_cover_destroyed)
        // GTW-507: the close-combat STRIKE flash at the struck target cell.
        .add_fx_reader::<MeleeResolved, _>(read_melee_resolved)
        // GTW-524: the fall-impact flash + "Fell" FCT pop at the landing cell; the FCT pop
        // reads the hot-reloadable FxTuning, composed onto the shared gate shape here.
        .add_fx_reader::<FallOccurred, _>(read_fall_occurred.run_if(resource_exists::<FxTuning>))
        // GTW-546: the grenade BLAST reader — seeds the PendingImpact `animate_impact` plays.
        .add_fx_reader::<ThrowResolved, _>(read_throw_resolved);
    let render_gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<EffectRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    // GTW-306: the firing FX. spawn_shot_projectiles spawns the traveling DIRECTIONAL
    // projectile off the ShotFired buffer; animate_impact drains NO buffer (it consumes the
    // handed-off PendingImpact entities). GTW-327 (slice 2): spawn_shot_projectiles ALSO
    // classifies each shot's FCT pops (classify_report) + anchor and threads them THROUGH the
    // projectile -> PendingImpact -> animate_impact pipeline, so each shot's numbers appear at
    // its own STAGGERED impact. Both READ the hot-reloadable FxTuning resource, so each adds it
    // to its gate so a pre-resolve frame (tuning not yet loaded) does not fail param validation.
    app.add_systems(
        Update,
        spawn_shot_projectiles
            .in_set(PresenterSystems::Overlay)
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
    // (the combat log keys its outcome lines off it). Its buffer is registered above via
    // `add_message` (idempotent), so the `MessageWriter` param is always valid
    // (`bevy-traps.md` #4) — no extra run gate is needed for the writer.
    .add_systems(
        Update,
        animate_impact
            .in_set(PresenterSystems::Overlay)
            .run_if(render_gate.and_then(resource_exists::<FxTuning>)),
    )
    // GTW-306: advance every traveling projectile + hand off its impact (unguarded — see the
    // fn doc's animator paragraph).
    .add_systems(
        Update,
        advance_projectiles.in_set(PresenterSystems::Overlay),
    )
    // GTW-302 (slice 2): rise + fade + despawn every live floating-combat-text pop (unguarded
    // animator). The reader slices SPAWN the pops; this is the generic animator the
    // primitive owns.
    .add_systems(
        Update,
        animate_floating_text.in_set(PresenterSystems::Overlay),
    )
    .add_systems(Update, expire_flashes.in_set(PresenterSystems::Overlay));
}

/// Registers the GTW-572 CONSEQUENCE-FCT PALETTE: the shared core (the per-frame
/// [`FctStackCounter`](crate::FctStackCounter) + its reset, explicitly ordered before the
/// reader set — `bevy-traps.md` #3, both inside the [`PresenterSystems::Overlay`] stage)
/// and then ONE registrar line per consequence family.
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

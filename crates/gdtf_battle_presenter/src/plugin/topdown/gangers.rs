//! Ganger sprite draw-band registration: spawn / move / appearance / life-state /
//! removal / tween / impact-despawn, plus the Compose-stage visibility resolver.

use bevy::{ecs::message::Messages, prelude::*};
use gdtf_battle_sim::{prelude::BattleInProgress, shot_fired::ShotFired};

use crate::{
    CharacterRoles, PresenterSystems, ShotImpactResolved, TopDownAtlases, advance_sprite_tweens,
    despawn_killed_ganger_on_impact, despawn_removed_ganger_sprites, move_ganger_sprites,
    resolve_ganger_appearance, resolve_ganger_visibility, spawn_ganger_sprites,
    update_ganger_life_state,
};

/// GTW-219 (S5): the ganger-draw change-detection systems join the
/// [`PresenterSystems::Scene`] stage (GTW-623 — the drawn-world stage; the Compose-stage
/// writers — the terrain fog and the GTW-627 visibility resolver — and the overlays order
/// after them by STAGE MEMBERSHIP, no pairwise edges). Each is gated
/// `run_if(resource_exists::<BattleInProgress>)` AND on
/// every render resource it reads — `CharacterRoles` (the data table) and
/// `TopDownAtlases` — so a `MinimalPlugins` headless app with no `AssetServer`
/// (those resources absent) simply does not draw rather than failing param
/// validation (`bevy-traps.md` #1; the ticket's "a no-resource state must NOT
/// panic the draw"). `ActiveLevel` + `GangerSprites` are `init_resource`-d on
/// build, so they are always present. `move_ganger_sprites` runs
/// `.after(spawn_ganger_sprites)` so a same-update spawn is already mapped when
/// the move runs — a TRUE intra-stage data-flow edge, KEPT (GTW-623 C2).
///
/// # THE one sim-owned `add_message` exception (GTW-623 C4 / A2)
///
/// GTW-331: `update_ganger_life_state` drains `MessageReader<ShotFired>` to discriminate a
/// shot-kill (deferred to the impact despawn) from a non-shot death (despawned promptly) —
/// see its docs. A `MessageReader` panics param validation without its `Messages<ShotFired>`
/// buffer (`bevy-traps.md` #4), and the ganger batch's gate is deliberately NOT extended
/// with a `Messages<ShotFired>` guard: the life-state system must keep running for the
/// non-shot death despawn even in a focused FIRE-LESS harness (the
/// `ganger_draw` / `fog_present` suites author gangers + `BattleInProgress` and register no
/// sim buffer at all), where such a gate would silently turn the whole ganger batch's
/// life-state handling off. So the presenter registers this ONE sim-owned buffer
/// idempotently here (`add_message` is a no-op when the sim's `BattleSimPlugin` already
/// registered it in a real battle). It is the SINGLE exception to the
/// never-`add_message`-a-sim-owned-buffer convention (GTW-572 C4 / GTW-623 C4) — pinned by
/// `fx_draw/registrar_contract.rs`; every other sim-owned buffer a presenter system drains
/// is gated `resource_exists::<Messages<M>>` and seeded by the sim (live play) or the
/// harness (tests).
///
/// # GTW-631 — the ONE appearance writer
///
/// `resolve_ganger_appearance` is the single writer of every ganger sprite's atlas index
/// AND tint, replacing the reframe / life-state re-tint / hot-reload re-index trio. Its
/// `CharacterRoles`-change driver (the dissolved GTW-375 re-index) lives INSIDE the
/// system as `roles.is_changed()` — a `resource_changed` run condition would gate its
/// changed-state and suppression-removal drivers off on roles-quiet frames — so the
/// registration needs no dedicated roles-change entry. It joins the same gated batch: it
/// requires `CharacterRoles` (in the gate, `bevy-traps.md` #1), and outside a battle /
/// without the atlases there are no mapped sprites to stamp, so the batch gate loses
/// nothing over the old re-index's looser table-only gate.
pub(super) fn register_ganger_draw(app: &mut App) {
    app.add_message::<ShotFired>();
    let gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<CharacterRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    app.add_systems(
        Update,
        (
            spawn_ganger_sprites,
            move_ganger_sprites.after(spawn_ganger_sprites),
            resolve_ganger_appearance,
            update_ganger_life_state,
        )
            .in_set(PresenterSystems::Scene)
            .run_if(gate),
    )
    // GTW-627: the ganger-visibility RESOLVER — the ONE writer of every ganger sprite's
    // `Visibility`, in the Compose stage (chained strictly after the Scene stage above by
    // STAGE MEMBERSHIP, GTW-623 — no pairwise `.after` edges). It resolves the drawn-band
    // storey fact AND, when the fog resources are resident, the fog fact through one pure
    // classifier; with the fog sets absent (a focused harness) it is band-only. Gated on
    // the battle witness alone: its classifier resources are `Option`al or
    // `init_resource`-d on build (`bevy-traps.md` #1), and with no mapped sprites it is
    // inert.
    .add_systems(
        Update,
        resolve_ganger_visibility
            .in_set(PresenterSystems::Compose)
            .run_if(resource_exists::<BattleInProgress>),
    )
    // The removal-detection despawn needs NO render resource (it only despawns
    // mapped sprites + drops map entries), so it is gated on the battle witness
    // alone — it must still run when the table / atlas happen to be absent so a
    // removed ganger never leaves an orphan sprite.
    .add_systems(
        Update,
        despawn_removed_ganger_sprites
            .in_set(PresenterSystems::Scene)
            .run_if(resource_exists::<BattleInProgress>),
    )
    // GTW-359 (AC3 / C4): the general sprite-movement glide. It ticks each ganger
    // sprite's `SpriteTween` (re-targeted by `move_ganger_sprites`) and writes the
    // interpolated `Transform.translation` IN PLACE — so a move is GLIDED, never
    // snapped. Ordered `.after(move_ganger_sprites)` so a same-frame re-target glides
    // this frame. Like `despawn_removed_ganger_sprites` it needs no render resource
    // (only `Res<Time>` + the `(Transform, SpriteTween)` query) and is inert with no
    // tweened sprites, so it is gated on the battle witness alone — an in-flight glide
    // still completes regardless of the table / atlas being present.
    .add_systems(
        Update,
        advance_sprite_tweens
            .in_set(PresenterSystems::Scene)
            .after(move_ganger_sprites)
            .run_if(resource_exists::<BattleInProgress>),
    )
    // GTW-331: the SHOT-KILL death-despawn. It drains the shared GTW-328
    // `ShotImpactResolved` signal `animate_impact` emits at each staggered impact and
    // despawns a ganger whose killing tracer just LANDED (its threaded report struck the
    // ganger + left it Dead) — so the body does not vanish at sim-drain time, before the
    // bolt reaches it. Like `despawn_removed_ganger_sprites` it only despawns mapped sprites
    // + drops map entries (no render resource), so it is gated on the battle witness AND its
    // `Messages<ShotImpactResolved>` buffer (a `MessageReader` panics validation without its
    // buffer — `bevy-traps.md` #4); the buffer is registered unconditionally in
    // `register_fx_flash_systems` (idempotent `add_message`), so the gate is satisfied
    // whenever a battle is live. The `Changed<LifeState>` path (above) defers a shot-kill to
    // this system, so the two never double-despawn (the map entry drops exactly once).
    .add_systems(
        Update,
        despawn_killed_ganger_on_impact
            .in_set(PresenterSystems::Scene)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<Messages<ShotImpactResolved>>),
            ),
    );
}

//! The [`CoverOnDeathRegistry`] resource + the [`resolve_on_death`] system — the on-death
//! MECHANICS resolver that fans each buffered death's authored effect generically through
//! the GTW-552 palette ([`crate::effects::on_death`]) (GTW-547, child GTW-41g).

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Entity, MessageReader, Query, Res, ResMut, Resource, With},
};

use super::{OnDeath, OnDeathEffect, OnDeathOccurred};
use crate::{
    effects::{
        fields::{FieldDefRegistry, FieldRegistry},
        on_death::{ApplyOnDeathEffect, DeathFanOut, VictimRow},
    },
    metric::CellLevel,
    occupancy::OccupancyGrid,
    weapon::{MeleeWeapon, MountedWeapon, Wields},
};

/// The **cover-cell → on-death-effect** map — the authored [`OnDeathEffect`] a destroyed piece
/// of cover fans when it is smashed (GTW-547).
///
/// Cover is NOT an entity (it lives in the [`CoverLedger`](crate::cover::CoverLedger) keyed by
/// cell), so a cover tile's authored on-death effect cannot ride a `Component` the way a
/// weapon's does. This battle-lifetime [`Resource`] holds it instead, keyed by the cover cell:
/// [`setup_battle`](crate::situation::setup_battle) seeds it from each cover piece's
/// [`TerrainDef::on_death`](crate::terrain::def::TerrainDef) field as it pours the cover ledger,
/// and [`resolve_on_death`] looks up a COVER death (an
/// [`OnDeathOccurred`] carrying [`Entity::PLACEHOLDER`]) by its
/// [`at`](OnDeathOccurred::at) cell. It PERSISTS for the battle lifetime (inserted on the same
/// `Ok` path as the other battle grids, removed at teardown), the
/// [`FieldRegistry`](crate::effects::fields::FieldRegistry) mirror.
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`CellLevel`]`, `[`OnDeathEffect`]`>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`). Private inner with
/// small accessors (the registry answers a cover-cell lookup, not a raw-map question — so no
/// derived `Deref`).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CoverOnDeathRegistry(HashMap<CellLevel, OnDeathEffect>);

impl CoverOnDeathRegistry {
    /// Build a cover-on-death registry from a `(cell, effect)` iterator — the shape the
    /// setup seed loop collects each cover piece's authored effect into.
    #[must_use]
    pub fn new(effects: impl IntoIterator<Item = (CellLevel, OnDeathEffect)>) -> Self {
        Self(effects.into_iter().collect())
    }

    /// Insert one cover cell's authored on-death effect, returning the previous effect at that
    /// cell (if any) — the per-piece insert the setup seed loop calls.
    pub fn insert(&mut self, at: CellLevel, effect: OnDeathEffect) -> Option<OnDeathEffect> {
        self.0.insert(at, effect)
    }

    /// Look up the authored on-death effect for a destroyed cover CELL, or [`None`] if the
    /// cell's cover authored none — the lookup [`resolve_on_death`] runs for a cover death.
    #[must_use]
    pub fn effect(&self, at: &CellLevel) -> Option<&OnDeathEffect> {
        self.0.get(at)
    }

    /// How many cover cells carry an authored on-death effect — the count a seed test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no cover cell carries an authored on-death effect.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// **Resolve** every buffered [`OnDeathOccurred`] — the GTW-547 on-death-effect applier
/// (the GTW-41 advanced-effects epic, child GTW-41g — a ticket-defined feature not yet
/// written into `docs/combat/resolution.md`).
///
/// Ordered AFTER every death producer (the ranged / melee fire dispatch, the DOT / field /
/// bleed clocks, the falls system, and both cover-destroy sites) so THIS frame's deaths are all
/// buffered when it runs (`bevy-traps.md` #3 — explicit `.after` on each; see
/// [`SimActsPlugin`](crate::acts::SimActsPlugin)'s wiring). For each buffered death:
///
/// 1. **Resolve the source's authored effect** — a GANGER death (a real
///    [`entity`](OnDeathOccurred::entity)) reads the [`OnDeath`] component off the ganger's
///    [`Wields`] weapon entity (the wielded-weapon spawn composed it from the weapon
///    spec's [`on_death`](crate::weapon::WeaponSpec) field); a COVER death (an
///    [`Entity::PLACEHOLDER`]) looks up the [`CoverOnDeathRegistry`] by the death
///    [`at`](OnDeathOccurred::at) cell. A source with no authored effect fans nothing.
/// 2. **Fan the effect at the death cell through the palette** — the effect enum's thin
///    delegation ([`ApplyOnDeathEffect`]) routes to the isolated per-effect behaviour in
///    [`crate::effects::on_death`] over the borrowed [`DeathFanOut`] surface. This resolver
///    NEVER matches the effect vocabulary (GTW-552) — the mechanics stay a work-queue + a
///    generic trait invocation.
///
/// # Chain-reaction cadence (the GTW-547 termination guarantee)
///
/// An [`OnDeathEffect::Explode`] fan can KILL more gangers (emptying their
/// [`Hp`](crate::ganger::Hp)), which must themselves fan their on-death effects — a cascading
/// explosion. This is resolved to a FIXPOINT WITHIN ONE system run: the buffered deaths seed a
/// local work-queue, and each fan that kills a ganger pushes that fresh death — a typed
/// [`OnDeathOccurred`] work item — onto the SAME queue via the surface's
/// [`cascade`](DeathFanOut::cascade) (NOT back through the message buffer, which the reader's
/// cursor would not re-observe this frame). A [`HashSet`] of already-processed death CELLS
/// guards re-entry, so each death is fanned exactly once and the cascade TERMINATES (bounded
/// by the finite live gangers / cover cells; a cover cell is destroyed once — its
/// [`deplete_cover`](crate::cover::CoverLedger::deplete_cover) `destroyed` flag is monotonic —
/// and a ganger flips [`LifeState::Dead`](crate::ganger::LifeState) once). No infinite loop,
/// no double-apply, all same-frame.
///
/// Param-only (`bevy-traps.md` #7 — no `&mut World`): a [`MessageReader`], the disjoint
/// [`Wields`] / [`OnDeath`] / [`VictimRow`] queries, and the
/// [`Res<OccupancyGrid>`] / [`ResMut<FieldRegistry>`] / [`Res<CoverOnDeathRegistry>`] world
/// reads plus an `Option<`[`Res<FieldDefRegistry>`]`>` (the field CATALOG is app/Load-owned, NOT
/// sim-`setup_battle`-inserted — a battle with no field content has none, so it is taken
/// `Option<Res>` and a `LeaveField` fans nothing when it is absent, `bevy-traps.md` #1). Pure,
/// render-free, saturating arithmetic — no underflow, no `unwrap`, no pixel.
#[expect(
    clippy::too_many_arguments,
    reason = "the resolver threads the death reader, the three disjoint weapon-resolution \
              queries (wields / melee / mounted markers) + the OnDeath read + the victim-surface \
              query, and the four battle-lifetime world resources (occupancy / field registry / \
              field catalog / cover-on-death registry) — each a distinct Bevy SystemParam (the \
              dispatch_fire / fold_ganger_round argument-count carve-out); bundling would only \
              hide the access set"
)]
pub fn resolve_on_death(
    mut deaths: MessageReader<OnDeathOccurred>,
    wields: Query<&Wields>,
    on_deaths: Query<&OnDeath>,
    melee: Query<(), With<MeleeWeapon>>,
    mounted: Query<(), With<MountedWeapon>>,
    mut victims: Query<VictimRow>,
    grid: Res<OccupancyGrid>,
    mut fields: ResMut<FieldRegistry>,
    field_defs: Option<Res<FieldDefRegistry>>,
    cover_on_death: Res<CoverOnDeathRegistry>,
) {
    // Seed the local work-queue from this frame's buffered deaths. Cascade deaths (a fan that
    // kills more) are pushed onto THIS queue, not re-emitted through the message buffer — the
    // reader's cursor would not re-observe them this frame. `visited` (keyed by death cell) is
    // the termination guard: a cell is fanned exactly once.
    let mut queue: Vec<OnDeathOccurred> = deaths.read().copied().collect();
    let mut visited: HashSet<CellLevel> = HashSet::new();

    while let Some(death) = queue.pop() {
        // Fan each death CELL at most once (the cascade-termination guard). Two overlapping
        // blasts, or an A-kills-B-kills-A geometry, cannot re-fan the same cell forever.
        if !visited.insert(death.at) {
            continue;
        }

        // (1) Resolve the dying source's authored effect. A ganger death reads OnDeath off its
        //     wielded weapon; a cover death (Entity::PLACEHOLDER) reads the cover registry by
        //     cell. A source with no authored effect fans nothing.
        let effect: Option<OnDeathEffect> = if death.entity == Entity::PLACEHOLDER {
            cover_on_death.effect(&death.at).cloned()
        } else {
            wields
                .get(death.entity)
                .ok()
                .and_then(|w| {
                    // The firing gun through the ONE shared preference rule (GTW-660,
                    // `Wields::firing_weapon`): prefer the mounted gun the ganger is
                    // manning, else its carried ranged weapon — the melee weapon is
                    // excluded so a gun's on-death effect is the one that fires, not
                    // the fists'.
                    w.firing_weapon(|e| mounted.get(e).is_ok(), |e| melee.get(e).is_ok())
                })
                .and_then(|weapon_entity| on_deaths.get(weapon_entity).ok())
                .map(|on_death| on_death.effect().clone())
        };
        let Some(effect) = effect else {
            continue;
        };

        // (2) Fan the effect at the death cell generically through the palette trait (the
        //     GTW-552 palette): the enum's thin delegation routes to the isolated per-effect
        //     behaviour over this borrowed surface — DIRECT, same-frame invocation (never a
        //     deferred command), so a lethal fan's kills land on `queue` before the next pop
        //     and the cascade cadence is unchanged. NO effect logic lives here.
        let mut fan_out = DeathFanOut {
            grid:       &grid,
            victims:    &mut victims,
            fields:     &mut fields,
            field_defs: field_defs.as_deref(),
            cascade:    &mut queue,
        };
        effect.fan_at(death.at, &mut fan_out);
    }
}

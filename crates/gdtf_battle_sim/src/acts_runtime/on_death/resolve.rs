//! The [`CoverOnDeathRegistry`] resource + the [`resolve_on_death`] system + the two
//! per-variant folder functions ([`explode`] / [`leave_field`]) (GTW-547, child GTW-41g).

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Entity, MessageReader, Query, Res, ResMut, Resource, With},
};

use super::{OnDeath, OnDeathEffect, OnDeathOccurred};
use crate::{
    fields::{FieldDefRegistry, FieldRegistry},
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    shot_pipeline::aoe::aoe_affected,
    weapon::{HitType, MeleeWeapon, MountedWeapon, Wields},
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
/// [`FieldRegistry`](crate::fields::FieldRegistry) mirror.
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

/// The victim-surface query row [`resolve_on_death`]'s [`explode`] drain mutates — the struck
/// ganger's `(`[`Hp`]`, `[`LifeState`]`)` bundled into one `QueryData` tuple alias so the
/// `Query` stays under clippy's type-complexity gate. Mutable on both (the blast drains
/// [`Hp`] and flips [`LifeState`] to [`LifeState::Dead`] on a lethal blast).
type VictimRow = (&'static mut Hp, &'static mut LifeState);

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
///    [`Wields`] weapon entity (the wielded-weapon scene seam composed it from the weapon
///    spec's [`on_death`](crate::weapon::WeaponSpec) field); a COVER death (an
///    [`Entity::PLACEHOLDER`]) looks up the [`CoverOnDeathRegistry`] by the death
///    [`at`](OnDeathOccurred::at) cell. A source with no authored effect fans nothing.
/// 2. **Run the matching per-variant folder function** at the death cell —
///    [`explode`] fans a GTW-541 [`aoe_affected`] blast (a flat, deterministic,
///    armor-bypassing, RNG-free [`Hp`] drain per ganger in the radius), and
///    [`leave_field`] spawns the referenced GTW-545 field.
///
/// # Chain-reaction cadence (the GTW-547 termination guarantee)
///
/// An [`explode`] blast can KILL more gangers (emptying their [`Hp`]), which must themselves
/// fan their on-death effects — a cascading explosion. This is resolved to a FIXPOINT WITHIN
/// ONE system run: the buffered deaths seed a local work-queue, and each blast that kills a
/// ganger pushes that fresh death onto the SAME queue (NOT back through the message buffer,
/// which the reader's cursor would not re-observe this frame). A [`HashSet`] of already-processed
/// death CELLS guards re-entry, so each death is fanned exactly once and the cascade TERMINATES
/// (bounded by the finite live gangers / cover cells; a cover cell is destroyed once — its
/// [`deplete_cover`](crate::cover::CoverLedger::deplete_cover) `destroyed` flag is monotonic —
/// and a ganger flips [`LifeState::Dead`] once). No infinite loop, no double-apply, all
/// same-frame.
///
/// Param-only (`bevy-traps.md` #7 — no `&mut World`): a [`MessageReader`], the disjoint
/// [`Wields`] / [`OnDeath`] / [`Hp`]+[`LifeState`] queries, and the
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
    // Seed the local work-queue from this frame's buffered deaths. Cascade deaths (a blast that
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
                    // Prefer the mounted gun the ganger is manning, else its carried ranged
                    // weapon (the fire-path resolution) — the melee weapon is excluded so a
                    // gun's on-death effect is the one that fires, not the fists'.
                    w.mounted_weapon(|e| mounted.get(e).is_ok())
                        .or_else(|| w.ranged_weapon(|e| melee.get(e).is_ok()))
                })
                .and_then(|weapon_entity| on_deaths.get(weapon_entity).ok())
                .map(|on_death| on_death.effect().clone())
        };
        let Some(effect) = effect else {
            continue;
        };

        // (2) Run the matching per-variant folder function at the death cell.
        match effect {
            OnDeathEffect::Explode {
                hit_type, damage, ..
            } => explode(death.at, hit_type, damage, &grid, &mut victims, &mut queue),
            OnDeathEffect::LeaveField { field } => {
                // The field catalog is app/Load-owned; with it absent a LeaveField fans nothing
                // (fail-closed, the tick_fields Option-resource precedent).
                if let Some(defs) = field_defs.as_deref() {
                    leave_field(death.at, &field, defs, &mut fields);
                }
            }
        }
    }
}

/// **Fan an `AoE` blast** at `at` — the [`OnDeathEffect::Explode`] folder function (GTW-547).
///
/// Enumerates the GTW-541 [`aoe_affected`] template's cell set (centred at `at`, which is also
/// its notional shooter origin so a cone degenerates to the full disc — a corpse has no fire
/// direction), reads each cell's [`OccupancyGrid::occupant`], and drains a flat, deterministic
/// [`ExplodeDamage`](super::ExplodeDamage) from each LIVE ganger's [`Hp`]
/// (`saturating_sub`, armor-bypassing, NO RNG — the [`tick_dot`](crate::dot::tick_dot) /
/// [`tick_fields`](crate::fields::tick_fields) direct-drain model). A blast that empties a
/// victim's [`Hp`] flips it to [`LifeState::Dead`] and pushes a fresh
/// [`OnDeathOccurred`] onto `queue` (the same-frame cascade — the caller's fixpoint loop
/// processes it, guarded by the visited-set). A corpse in the radius is skipped (already Dead).
///
/// Faction-BLIND (the GTW-541 friendly-fire property `resolution.md` §2): the blast strikes
/// EVERY occupant in the radius, including allies. Deterministic — [`aoe_affected`] returns a
/// canonically-sorted set and the drain takes no RNG, so a demo explosion is byte-stable.
fn explode(
    at: CellLevel,
    hit_type: HitType,
    damage: super::ExplodeDamage,
    grid: &OccupancyGrid,
    victims: &mut Query<VictimRow>,
    queue: &mut Vec<OnDeathOccurred>,
) {
    // The blast centre is BOTH the impact and the notional shooter (a corpse has no fire
    // direction), so a Cone falls back to the full disc — the documented degenerate fallback.
    for cell in aoe_affected(at, hit_type, at) {
        let Some(occupant) = grid.occupant(&cell) else {
            continue;
        };
        let Ok((mut hp, mut life)) = victims.get_mut(occupant) else {
            continue;
        };
        // Skip a corpse (already Dead) — it neither takes damage nor re-fans (the once-only
        // property the visited-set also enforces at the death-cell level).
        if *life == LifeState::Dead {
            continue;
        }
        // Flat, armor-bypassing, RNG-free drain (saturating at 0 — Hp is unsigned).
        *hp = Hp::new(hp.saturating_sub(*damage));
        // A blast that empties Hp KILLS (the DOT / field-kill precedent) and re-emits the
        // cascade death onto the caller's work-queue (same-frame, visited-set-guarded).
        if *hp == Hp::new(0) {
            *life = LifeState::Dead;
            queue.push(OnDeathOccurred::new(occupant, cell));
        }
    }
}

/// **Leave a persistent field** at `at` — the [`OnDeathEffect::LeaveField`] folder function
/// (GTW-547).
///
/// Resolves the authored [`FieldKey`](crate::fields::FieldKey) against the
/// [`FieldDefRegistry`] to its [`FieldDef`](crate::fields::FieldDef) and calls the GTW-545
/// [`FieldRegistry::spawn`] placement API at `at`, so the cell becomes a live hazard that
/// persists + ticks per GTW-545 rules. An unresolvable key (no field file with that stem
/// loaded) fans nothing (fail-closed — no panic), the setup-time
/// [`FieldNotFound`](crate::situation::BattleSetupError) abort's runtime counterpart.
fn leave_field(
    at: CellLevel,
    field: &crate::fields::FieldKey,
    field_defs: &FieldDefRegistry,
    fields: &mut FieldRegistry,
) {
    let Some(def) = field_defs.def(field) else {
        return; // no such field loaded — fail closed, no panic
    };
    fields.spawn(at, def.clone());
}

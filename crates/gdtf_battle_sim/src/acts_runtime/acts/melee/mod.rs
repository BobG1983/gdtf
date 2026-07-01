//! The **melee** dispatch — drains a buffered [`MeleeRequested`], gates 8-adjacency (+ clear
//! LOS + an alive opposing target on the ganger arm), spends the wielded melee weapon's primary
//! fight-mode TU, and branches on the [`MeleeTarget`]: the §7 opposed-Fight → §5 damage → §6
//! wound synthesis onto a ganger (GTW-507), or the UNCONTESTED cover-smash onto an adjacent
//! inert STRUCTURE (GTW-508, child GTW-37d of the GTW-37 melee epic; `docs/combat/resolution.md`
//! §7).
//!
//! No combat math is reimplemented here: the ganger path REUSES the GTW-506 opposed-Fight
//! core ([`opposed_fight`](crate::melee::opposed_fight) /
//! [`melee_damage_mult`](crate::melee::melee_damage_mult) /
//! [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier)) and the existing §4/§5/§6
//! pieces verbatim, composed by [`resolve_melee_strike`](crate::melee::resolve_melee_strike);
//! the structural path REUSES [`resolve_structural_melee`](crate::melee::resolve_structural_melee)
//! (multiplied weapon damage through the EXISTING cover ledger). The gates REUSE
//! [`is_8_adjacent`](crate::downed_acts::is_8_adjacent) and [`has_los`](crate::los::has_los)
//! verbatim. Param-only (`bevy-traps.md` #7 — no `&mut World`); the disjoint queries coexist
//! with no `B0001` conflict (see the per-query docs).
//!
//! ## Module layout (GTW-508 C6 — code-health size cap)
//!
//! This module (`mod.rs`) owns the [`dispatch_melee`] system + its query `type` aliases + the
//! [`MeleeGrids`] / [`MeleeRngs`] [`SystemParam`](bevy::ecs::system::SystemParam) bundles; the
//! two per-target arm resolvers ([`resolve`]) live in the sibling submodule so each concern
//! stays under the code-health file-size cap.

use bevy::{
    ecs::system::SystemParam,
    prelude::{MessageReader, MessageWriter, Query, Res, ResMut, With},
};

use crate::{
    acts::request::{MeleeRequested, MeleeResolved, MeleeTarget, ShoveRequested},
    armor::{PieceArmorMut, Wears, WornBy},
    cover::CoverLedger,
    fire::{MeleeQuery, WieldsQuery},
    ganger::{
        Facing, Faction, Fight, Hp, LifeState, Luck, Position, Stance, Toughness, Tu, Wounds,
    },
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    melee::MeleeWeaponHit,
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::CoverDestroyed,
    rng::{FightRng, SeverityRng, ShotRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, FightMode, Shove, WeaponDamage, WeaponPunch, WeaponShred},
};

/// The read-only geometry/stat snapshot query — every `Copy` read the gates + the §7 opposed
/// roll need off BOTH the attacker and the target ganger, factored into a `type` so
/// [`dispatch_melee`] stays under clippy's type-complexity gate.
///
/// Reads `Position` / `Stance` / `Facing` / `Fight` / `Faction` / `Luck` — all IMMUTABLE, so
/// this query is disjoint from the mutable [`MeleeTargetQuery`] (which writes
/// `Hp`/`Wounds`/`LifeState`/`InflictedWounds`, a different mutable set; `Luck` is read-only in
/// both, which never conflicts) and the attacker's `&mut Tu` query — all coexist with no
/// `B0001` conflict. The system snapshots its reads as `Copy` values before any mutation, so
/// the gating reads (8-adjacency, LOS, faction, the two Fights) are taken once up front.
type MeleeGeomQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static Position,
        &'static Stance,
        &'static Facing,
        &'static Fight,
        &'static Faction,
        &'static Luck,
    ),
>;

/// The mutable TARGET surfaces query — the struck ganger's four `&mut` battle surfaces plus
/// the read attribute stats + its injury ledger the §6 fold needs, factored into a `type`
/// (the [`crate::fire::TargetQuery`] precedent).
///
/// Mutable on `Hp`/`Wounds`/`LifeState`/`InflictedWounds` (the [`apply_hit`](crate::apply_hit::apply_hit)
/// fold writes these) and read-only on `Toughness`/`Luck`/`InflictedInjuries` (the §6 severity
/// inputs). Disjoint from [`MeleeGeomQuery`] (no shared MUTABLE component) and the attacker's
/// `&mut Tu` query (a different mutable component), so no `ParamSet` is needed. Used via
/// `get_mut(target)` after the gates pass — the alive gate already read the target's snapshotted
/// `LifeState` from [`MeleeGeomQuery`], so this only takes the exclusive borrow once, at apply
/// time.
type MeleeTargetQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
        Option<&'static InflictedInjuries>,
    ),
>;

/// The wielded MELEE-weapon stat query — the per-hit §5 damage columns, the §6 [`FatalBias`],
/// and the [`FightMode`] selector the dispatch reads off the related melee weapon entity,
/// factored into a `type` (the [`crate::fire::WeaponQuery`] precedent).
///
/// Read-only over the MELEE weapon entities (resolved `attacker → Wields → the MeleeWeapon
/// entity`), disjoint from every ganger-entity query above — so it coexists with no `B0001`
/// conflict. Assembled into a [`MeleeWeaponHit`] borrow-view + read for the primary fight-mode
/// TU cost.
type MeleeWeaponQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static WeaponDamage,
        &'static WeaponPunch,
        &'static WeaponShred,
        &'static DamageType,
        &'static FatalBias,
        &'static FightMode,
        // GTW-525: the `shove` knockback tag — a connecting strike with it auto-shoves the target.
        &'static Shove,
    ),
>;

/// The change-driven world grids + tuning the [`has_los`] gate reads, bundled into one
/// [`SystemParam`] so [`dispatch_melee`] stays under Bevy's 16-param system limit (the
/// [`crate::acts::BattleGridsParam`] grouping precedent).
///
/// The three grids the LOS march flies through ([`OccupancyGrid`] / [`SurfaceGrid`] /
/// [`CoverLedger`]) + the [`CombatTuning`] all the §4/§5/§6/§7 reads consume — every one a
/// battle-lifetime `Res<T>` (the band's `BattleInProgress` `run_if` keeps them present). A
/// transparent system-param bundle of named world-state resources, not itself a wrapped domain
/// scalar.
#[derive(SystemParam)]
pub struct MeleeGrids<'w> {
    /// The coarse 3D occupancy grid — the LOS march's collision / occupant-band surface (also
    /// the stair-eye-offset lookup for the observer eye).
    occupancy: Res<'w, OccupancyGrid>,
    /// The persistent floor/roof-slab + ground surface grid the LOS march flies through.
    surface:   Res<'w, SurfaceGrid>,
    /// The model cover ledger — peeked (read only) for the LOS march's cover bands on the
    /// ganger path, and SPENT (the depletion writer) on the GTW-508 cover-smash path. A
    /// `ResMut` because the structural strike arm calls
    /// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover) — the LOS march
    /// reads it through `&*` (a `ResMut` derefs to `&CoverLedger`), so the ganger path is
    /// unchanged.
    cover:     ResMut<'w, CoverLedger>,
    /// The combat tuning the §4 body-part weights, §5 damage formula, §6 severity scaling, §7
    /// melee curve, and the LOS view geometry all read.
    tuning:    Res<'w, CombatTuning>,
}

/// The three seeded draw streams the §7 / §4 / §6 melee synthesis advances, bundled into one
/// [`SystemParam`] so [`dispatch_melee`] stays under Bevy's 16-param system limit.
///
/// Every field is a [`ResMut`] (drawing advances the cursor — never `Res`, the `rng::streams`
/// binding constraint): [`FightRng`] (the two §7 opposed-Fight rolls), [`ShotRng`] (the §4
/// body-part roll), [`SeverityRng`] (the §6 severity term). A transparent system-param bundle
/// of the named stream resources, not itself a wrapped domain scalar. All three are battle-set
/// (inserted at setup alongside the other streams), so the band's `BattleInProgress` `run_if`
/// keeps them present.
/// The three seeded draw streams the §7 / §4 / §6 melee synthesis advances, each taken
/// `Option<ResMut<…>>` so a focused harness that opens `BattleInProgress` WITHOUT the full setup
/// flow (the fire/cover bridge tests insert only the streams `dispatch_fire` needs) does not
/// panic this runtime system on a stream's absence (`bevy-traps.md` #1; the `reaction_trigger`
/// `Option<ResMut<ReactionRng>>` precedent). With ANY of the three absent, [`dispatch_melee`]
/// resolves no strike (a safe, defined fallback — never a panic). In the real app all three are
/// sim-set (inserted at `setup_battle`), so the live melee act always has them.
#[derive(SystemParam)]
pub struct MeleeRngs<'w> {
    /// The §7 opposed-Fight stream — two draws per resolve (GTW-506).
    fight:    Option<ResMut<'w, FightRng>>,
    /// The §4 body-part-roll stream — one draw per resolve.
    shot:     Option<ResMut<'w, ShotRng>>,
    /// The §6 severity-roll stream — one draw per CONNECTING resolve (zero on a miss).
    severity: Option<ResMut<'w, SeverityRng>>,
}

/// The ground-plane [`Cell`] of a [`Position`] — its `(x, y)` (the `actor_cell` / `row_cell`
/// split precedent in `fire.rs` / `trigger.rs`).
fn ganger_cell(position: &Position) -> Cell {
    let key = ***position;
    Cell::new(key.x, key.y)
}

/// The storey [`Level`] of a [`Position`] — its `z` storey index (the `trigger.rs` `row_level`
/// precedent).
fn ganger_level(position: &Position) -> Level {
    let key = ***position;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is a storey index in 0..MAX_LEVELS (8) by construction, so the i32 -> u8 \
                  narrowing cannot truncate or sign-flip (the trigger.rs row_level precedent)"
    )]
    let storey = key.z as u8;
    Level::new(storey)
}

/// **Dispatch** buffered [`MeleeRequested`] messages — the LIVE melee act (GTW-507).
///
/// For each request the dispatch:
///
/// 1. **Snapshots** the attacker's + target's `Copy` reads off the geometry query
///    (position / stance / facing / Fight / faction / Luck), releasing the read borrow before
///    any mutation. An attacker or target missing the read components is skipped (fail-closed,
///    no panic).
/// 2. **Gates** the strike, REUSING the landed predicates verbatim (reject = no spend, no
///    strike, no signal — never a panic):
///    - [`is_8_adjacent`](crate::downed_acts::is_8_adjacent) — same-storey Moore-8 reach (the §7
///      close-combat range this slice; reach > 1 is a future refinement);
///    - the target is [`LifeState::is_active`] (an ALIVE — incl. Downed — opposing ganger; a
///      corpse is no target);
///    - `attacker.faction != target.faction` — an OPPOSING ganger (no friendly melee);
///    - [`has_los`](crate::los::has_los) — a clear sight line attacker → target over the SAME
///      voxel geometry the sim fires through (`docs/combat/resolution.md` §2 clearance), built
///      exactly as `reaction_trigger` builds it (the corpse `is_dead` pass-through reused).
/// 3. **Resolves the weapon** — `attacker → Wields → the MeleeWeapon entity`
///    ([`Wields::melee_weapon`](crate::weapon::Wields::melee_weapon), excluding the ranged
///    weapon via the [`MeleeQuery`] probe), reading its §5 damage stats + primary
///    [`FightMode`] TU cost. An attacker wielding no melee weapon is skipped (fail-closed).
/// 4. **Spends** the wielded melee weapon's primary fight-mode flat TU off the attacker's
///    `&mut Tu` (saturating; the act is taken whether or not the strike connects — a swing
///    costs TU regardless, mirroring the ranged `fire()` TU charge).
/// 5. **Runs** [`resolve_melee_strike`](crate::melee::resolve_melee_strike) — the §7
///    opposed-Fight → §5 damage (× the §7 margin
///    multiplier) → §6 wound synthesis (GTW-506 core + the §4/§5/§6 pieces, reused verbatim),
///    folding the connecting hit onto the target's `&mut` surfaces in place and emitting the
///    existing wound/injury signals (via [`apply_hit`](crate::apply_hit::apply_hit) +
///    `InflictedWounds`). The three injected streams (`FightRng` / `ShotRng` / `SeverityRng`)
///    are the only entropy.
/// 6. On a **connect**, emits ONE [`MeleeResolved`] carrying the target's struck `(cell, level)`
///    plus the weapon's [`DamageType`] (the presenter's strike-glyph FX role/color). A MISS
///    (the opposed roll lost) deals no damage and emits nothing.
///
/// # Melee-vs-STRUCTURE (GTW-508)
///
/// When the [`MeleeTarget`] is a [`Structure`](MeleeTarget::Structure) cell instead of a
/// ganger, the dispatch resolves an UNCONTESTED cover-smash: it gates only 8-adjacency to the
/// struck cell (LOS to an immediately-adjacent structure is trivially satisfied — no spurious
/// LOS block), spends the same wielded fight-mode TU, and calls
/// [`resolve_structural_melee`](crate::melee::resolve_structural_melee) — which applies
/// multiplied (`mult_max`, FORK 4a) weapon damage through the EXISTING
/// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover). There is **NO
/// opposed Fight roll and NO [`FightRng`] draw** (nor any `ShotRng` / `SeverityRng` draw) — a
/// structure is inert. On a lethal smash ([`CoverEvent::Destroyed`](crate::cover::CoverEvent::Destroyed))
/// it emits the EXISTING
/// [`CoverDestroyed`] signal (the GTW-386 FX fires verbatim); on either outcome it emits a
/// [`MeleeResolved`] strike-glyph at the struck cell. A cell with no ledger entry lazy-seeds at
/// its authored `max_hp` (the ledger owns that), so a strike on an unauthored cell still
/// resolves defined bookkeeping (never a panic).
///
/// Ordered in [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems) by
/// [`SimActsPlugin`](crate::acts::SimActsPlugin). Param-only (`bevy-traps.md` #7 — no
/// `&mut World`); the queries are disjoint (see each `type`'s doc). The `ResMut<FightRng>` /
/// `ResMut<ShotRng>` / `ResMut<SeverityRng>` are the drawing streams (never `Res` — drawing
/// advances the cursor; the `rng::streams` binding constraint).
#[expect(
    clippy::too_many_arguments,
    reason = "the melee dispatch needs the request reader, the disjoint geometry / attacker-Tu \
              / target-surfaces ganger queries, the two armor relationship queries, the wields \
              + melee-marker + melee-weapon-stat relationship queries, the grouped grids+tuning \
              (MeleeGrids) + draw streams (MeleeRngs) bundles, and the MeleeResolved + \
              CoverDestroyed + ShoveRequested writers — each a distinct Bevy SystemParam (the \
              dispatch_fire argument-count carve-out)"
)]
pub fn dispatch_melee(
    mut requests: MessageReader<MeleeRequested>,
    geom: MeleeGeomQuery,
    mut tu_q: Query<&mut Tu>,
    mut targets: MeleeTargetQuery,
    // Worn-armor relationship queries (the struck piece resolution, GTW-323 / ADR-0004) —
    // disjoint from the ganger queries (a different component / a different entity set).
    wears: Query<&Wears>,
    mut pieces: Query<PieceArmorMut, With<WornBy>>,
    // The wielded-weapon relationship — `attacker → Wields → the melee weapon entity`.
    wields: WieldsQuery,
    melee: MeleeQuery,
    weapons: MeleeWeaponQuery,
    // The LOS gate's read grids + tuning, grouped (MeleeGrids) so the system stays under
    // Bevy's 16-param limit. `cover` is `ResMut` (the GTW-508 cover-smash spends it).
    mut grids: MeleeGrids,
    // The three draw streams the §7 / §4 / §6 synthesis advances, grouped (MeleeRngs).
    rngs: MeleeRngs,
    mut resolved: MessageWriter<MeleeResolved>,
    // The GTW-508 cover-destroyed bridge — a lethal cover-smash writes the EXISTING signal the
    // presenter's `read_cover_destroyed` FX (GTW-386) already reacts to (`bevy-traps.md` #4).
    mut cover_destroyed: MessageWriter<CoverDestroyed>,
    // GTW-525: the weapon-tag auto-shove bridge — a connecting ganger strike with a `shove`
    // weapon writes an internal ShoveRequested (ShoveSource::Weapon) `dispatch_shove` drains
    // this same frame (it is ordered `.after(dispatch_melee)`); a miss / non-`shove` weapon
    // writes nothing (`bevy-traps.md` #4).
    mut shoves: MessageWriter<ShoveRequested>,
) {
    // `bevy-traps.md` #1: without all three seeded streams no strike can resolve a draw — fail
    // closed (no panic) rather than reading an absent battle-lifetime resource. In the real app
    // all three are sim-set (inserted at setup), so this never bails there (the `reaction_trigger`
    // `Option<ResMut<ReactionRng>>` precedent). The structural (cover-smash) path takes NO draw,
    // but the streams are always present in a live battle, so gating both arms here is harmless.
    let (Some(mut fight_rng), Some(mut shot_rng), Some(mut severity_rng)) =
        (rngs.fight, rngs.shot, rngs.severity)
    else {
        return;
    };
    for request in requests.read() {
        // (1) Snapshot the attacker's gating reads (Copy), releasing the read borrow before the
        //     mutations. A missing read component fails the strike (fail-closed). Shared by both
        //     the ganger-vs-ganger and the melee-vs-structure paths.
        let Ok((&atk_pos, &atk_stance, &atk_facing, &atk_fight, &atk_faction, &atk_luck)) =
            geom.get(request.attacker)
        else {
            continue;
        };

        // (2) Resolve the wielded MELEE weapon (`attacker → Wields → the MeleeWeapon entity`,
        //     excluding the ranged weapon), and read its §5 stats + primary fight-mode TU —
        //     shared by both paths. An attacker wielding no melee weapon cannot strike
        //     (fail-closed).
        let Some(weapon_entity) = wields
            .get(request.attacker)
            .ok()
            .and_then(|w| w.melee_weapon(|entity| melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((damage, punch, shred, damage_type, fatal_bias, fight_mode, shove)) =
            weapons.get(weapon_entity)
        else {
            continue;
        };
        let weapon = MeleeWeaponHit {
            damage,
            punch,
            shred,
            damage_type,
            fatal_bias,
        };
        // The primary fight-mode flat TU cost (the wielded weapon's first authored mode, or a
        // total structural default — `FightMode::primary` never panics).
        let tu_cost = Tu::new(u8::try_from(*fight_mode.primary().tu_cost).unwrap_or(u8::MAX));
        let strike_damage_type: DamageType = *damage_type;

        // The attacker's snapshotted gating reads + the resolved weapon/TU, bundled for the
        // per-arm resolvers (below) so the branch stays a two-line dispatch.
        let attacker = AttackerSnapshot {
            entity: request.attacker,
            position: atk_pos,
            stance: atk_stance,
            facing: atk_facing,
            fight: atk_fight,
            faction: atk_faction,
            luck: atk_luck,
            weapon,
            tu_cost,
            strike_damage_type,
            shove: *shove,
        };
        match request.target {
            // ── Ganger-vs-ganger (GTW-506/507) — the contested opposed-Fight path (unchanged). ──
            MeleeTarget::Ganger(target_entity) => resolve_ganger_melee(
                &attacker,
                target_entity,
                &geom,
                &mut tu_q,
                &mut targets,
                &wears,
                &mut pieces,
                &mut grids,
                MeleeStreams {
                    fight:    &mut fight_rng,
                    shot:     &mut shot_rng,
                    severity: &mut severity_rng,
                },
                &mut resolved,
                &mut shoves,
            ),

            // ── Melee-vs-structure (GTW-508) — the UNCONTESTED cover-smash path. ──
            MeleeTarget::Structure(at) => resolve_structure_melee(
                &attacker,
                at,
                &mut tu_q,
                &mut grids,
                &mut resolved,
                &mut cover_destroyed,
            ),
        }
    }
}

/// The per-target melee resolvers — the two arm helpers [`dispatch_melee`] branches into
/// (GTW-508 C6 — split out to keep each concern under the code-health size cap).
mod resolve;

use resolve::{AttackerSnapshot, MeleeStreams, resolve_ganger_melee, resolve_structure_melee};

//! The **melee** dispatch — drains a buffered [`MeleeRequested`], gates 8-adjacency +
//! clear LOS + an alive opposing target, spends the wielded melee weapon's primary
//! fight-mode TU, and runs the §7 opposed-Fight → §5 damage → §6 wound synthesis
//! (GTW-507, child GTW-37c of the GTW-37 melee epic; `docs/combat/resolution.md` §7).
//!
//! No combat math is reimplemented here: the resolution REUSES the GTW-506 opposed-Fight
//! core ([`opposed_fight`](crate::melee::opposed_fight) /
//! [`melee_damage_mult`](crate::melee::melee_damage_mult) /
//! [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier)) and the existing §4/§5/§6
//! pieces verbatim, composed by [`resolve_melee_strike`]; the gates REUSE
//! [`is_8_adjacent`](crate::downed_acts::is_8_adjacent) and [`has_los`](crate::los::has_los)
//! verbatim. Param-only (`bevy-traps.md` #7 — no `&mut World`); the disjoint queries coexist
//! with no `B0001` conflict (see the per-query docs).

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, MessageReader, MessageWriter, Query, Res, ResMut, With},
};

use crate::{
    acts::request::{MeleeRequested, MeleeResolved},
    armor::{PieceArmorMut, Wears, WornBy},
    cover::CoverLedger,
    downed_acts::is_8_adjacent,
    fire::{MeleeQuery, WieldsQuery},
    ganger::{
        Facing, Faction, Fight, Hp, LifeState, Luck, Position, Stance, Toughness, Tu, Wounds,
        effective_luck, effective_toughness,
    },
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    los::{Observer, PeekOffset, Target, has_los},
    melee::{Combatants, MeleeStrike, MeleeWeaponHit, resolve_melee_strike},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{FightRng, SeverityRng, ShotRng},
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, FightMode, WeaponDamage, WeaponPunch, WeaponShred},
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
    /// The model cover ledger — peeked (read only) for the LOS march's cover bands. The melee
    /// path never SPENDS cover (it strikes a ganger, not cover — cover-smash is GTW-508), so
    /// this is `Res`, not `ResMut`.
    cover:     Res<'w, CoverLedger>,
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
///    - [`is_8_adjacent`] — same-storey Moore-8 reach (the §7 close-combat range this slice;
///      reach > 1 is a future refinement);
///    - the target is [`LifeState::is_active`] (an ALIVE — incl. Downed — opposing ganger; a
///      corpse is no target);
///    - `attacker.faction != target.faction` — an OPPOSING ganger (no friendly melee);
///    - [`has_los`] — a clear sight line attacker → target over the SAME voxel geometry the
///      sim fires through (`docs/combat/resolution.md` §2 clearance), built exactly as
///      `reaction_trigger` builds it (the corpse `is_dead` pass-through reused).
/// 3. **Resolves the weapon** — `attacker → Wields → the MeleeWeapon entity`
///    ([`Wields::melee_weapon`](crate::weapon::Wields::melee_weapon), excluding the ranged
///    weapon via the [`MeleeQuery`] probe), reading its §5 damage stats + primary
///    [`FightMode`] TU cost. An attacker wielding no melee weapon is skipped (fail-closed).
/// 4. **Spends** the wielded melee weapon's primary fight-mode flat TU off the attacker's
///    `&mut Tu` (saturating; the act is taken whether or not the strike connects — a swing
///    costs TU regardless, mirroring the ranged `fire()` TU charge).
/// 5. **Runs** [`resolve_melee_strike`] — the §7 opposed-Fight → §5 damage (× the §7 margin
///    multiplier) → §6 wound synthesis (GTW-506 core + the §4/§5/§6 pieces, reused verbatim),
///    folding the connecting hit onto the target's `&mut` surfaces in place and emitting the
///    existing wound/injury signals (via [`apply_hit`](crate::apply_hit::apply_hit) +
///    `InflictedWounds`). The three injected streams (`FightRng` / `ShotRng` / `SeverityRng`)
///    are the only entropy.
/// 6. On a **connect**, emits ONE [`MeleeResolved`] carrying the target's struck `(cell, level)`
///    plus the weapon's [`DamageType`] (the presenter's strike-glyph FX role/color). A MISS
///    (the opposed roll lost) deals no damage and emits nothing.
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
              (MeleeGrids) + draw streams (MeleeRngs) bundles, and the MeleeResolved writer — \
              each a distinct Bevy SystemParam (the dispatch_fire argument-count carve-out)"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the dispatch is ONE cohesive per-request pass (snapshot the two gangers → gate \
              adjacency + faction + alive + LOS → resolve the wielded melee weapon → spend the \
              fight-mode TU → run the synthesis → emit on a connect); splitting it would thread \
              the grids/tuning/RNG bundles + the borrowed is_dead closure through helpers and \
              obscure the access set more than the length costs (the reaction_trigger / \
              dispatch_fire precedent)"
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
    // Bevy's 16-param limit.
    grids: MeleeGrids,
    // The three draw streams the §7 / §4 / §6 synthesis advances, grouped (MeleeRngs).
    rngs: MeleeRngs,
    mut resolved: MessageWriter<MeleeResolved>,
) {
    // `bevy-traps.md` #1: without all three seeded streams no strike can resolve a draw — fail
    // closed (no panic) rather than reading an absent battle-lifetime resource. In the real app
    // all three are sim-set (inserted at setup), so this never bails there (the `reaction_trigger`
    // `Option<ResMut<ReactionRng>>` precedent).
    let (Some(mut fight_rng), Some(mut shot_rng), Some(mut severity_rng)) =
        (rngs.fight, rngs.shot, rngs.severity)
    else {
        return;
    };
    for request in requests.read() {
        // (1) Snapshot the attacker's + target's gating reads (Copy), releasing the read borrow
        //     before the mutations. A missing read component fails the strike (fail-closed).
        let Ok((&atk_pos, &atk_stance, &atk_facing, &atk_fight, &atk_faction, &atk_luck)) =
            geom.get(request.attacker)
        else {
            continue;
        };
        let Ok((&tgt_pos, &tgt_stance, _, &tgt_fight, &tgt_faction, _)) = geom.get(request.target)
        else {
            continue;
        };

        // (2) Gate — 8-adjacency + opposing faction (the cheap reads first).
        if !is_8_adjacent(atk_pos, tgt_pos) || atk_faction == tgt_faction {
            continue;
        }

        // The target's liveness — an ALIVE (incl. Downed) opposing ganger only; a corpse is no
        // target. Read off the target query (the SINGLE LifeState read path), snapshotted.
        let Ok((_, _, &tgt_life, _, &tgt_toughness, &tgt_luck, tgt_injuries)) =
            targets.get(request.target)
        else {
            continue;
        };
        if !tgt_life.is_active() {
            continue;
        }
        // Route the defender's Toughness / Luck through the gate-enforced effective accessors
        // over its injury ledger (an absent ledger = the zero-delta identity), so an injury
        // shifting either stat shifts the §6 severity roll in step (the `fold_ganger_round`
        // precedent).
        let (effective_toughness, effective_luck) = match tgt_injuries {
            Some(ledger) => (
                effective_toughness(tgt_toughness, ledger),
                effective_luck(tgt_luck, ledger),
            ),
            None => (tgt_toughness, tgt_luck),
        };

        // The LOS gate — a clear sight line attacker → target over the SAME voxel geometry the
        // sim fires through. Built exactly as `reaction_trigger` builds the observer/target.
        let observer = Observer {
            position:         &atk_pos,
            stance:           &atk_stance,
            facing:           &atk_facing,
            stair_eye_offset: grids.occupancy.stair_eye_offset_at(&atk_pos),
            peek_offset:      PeekOffset::default(),
        };
        let los_target = Target {
            position: &tgt_pos,
            stance:   &tgt_stance,
        };
        // A Dead ganger is a corpse the LOS march flies THROUGH (GTW-317) — the same `is_dead`
        // pass-through `has_los`/`can_see` take. The predicate borrows `targets` IMMUTABLY only
        // for this `has_los` call; it is dropped before the `&mut targets` fold below (NLL), so
        // the immutable + mutable accesses never overlap. An entity absent from `targets` is no
        // corpse (defaults `false`).
        let is_dead = |entity: Entity| {
            targets
                .get(entity)
                .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
        };
        let sighted = has_los(
            &observer,
            &los_target,
            &grids.occupancy,
            &grids.surface,
            &grids.cover,
            &grids.tuning,
            is_dead,
        );
        if !*sighted {
            continue;
        }

        // (3) Resolve the wielded MELEE weapon (`attacker → Wields → the MeleeWeapon entity`,
        //     excluding the ranged weapon), and read its §5 stats + primary fight-mode TU. An
        //     attacker wielding no melee weapon cannot strike (fail-closed).
        let Some(weapon_entity) = wields
            .get(request.attacker)
            .ok()
            .and_then(|w| w.melee_weapon(|entity| melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((damage, punch, shred, damage_type, fatal_bias, fight_mode)) =
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

        // (4) Spend the fight-mode TU off the attacker (saturating). The swing costs TU whether
        //     or not it connects (the ranged `fire()` TU-charge precedent). A missing Tu pool
        //     fails the strike (fail-closed).
        let Ok(mut attacker_tu) = tu_q.get_mut(request.attacker) else {
            continue;
        };
        spend_tu(&mut attacker_tu, tu_cost);

        // (5) Run the §7 → §5 → §6 synthesis onto the target. The verb owns the §4 part roll,
        //     so the struck piece cannot be keyed by part before it runs; `strike_with_target`
        //     resolves the target's worn protection as the verb's armor input (bare flesh if the
        //     target wears nothing) and re-borrows the target's `&mut` surfaces for the fold
        //     (the snapshot reads above are released — `get_mut` takes a fresh exclusive borrow).
        let strike = strike_with_target(
            &mut targets,
            &wears,
            &mut pieces,
            request.target,
            Combatants {
                attacker_fight: atk_fight,
                defender_fight: tgt_fight,
                attacker_luck:  atk_luck,
            },
            effective_toughness,
            effective_luck,
            weapon,
            &grids.tuning,
            &mut fight_rng,
            &mut shot_rng,
            &mut severity_rng,
        );

        // (6) On a connect, emit the presenter strike-glyph signal at the target's cell. A miss
        //     deals no damage and emits nothing (the §7 connect gate).
        if strike.connect {
            let at = CellLevel::new(ganger_cell(&tgt_pos), ganger_level(&tgt_pos));
            resolved.write(MeleeResolved::new(at, strike_damage_type));
        }
    }
}

/// Fold one melee strike onto the target — resolve the struck worn piece, assemble the
/// [`TargetGanger`] borrow-view, and run [`resolve_melee_strike`] (split out so
/// [`dispatch_melee`] stays under clippy's line gate).
///
/// The §4 part roll lives INSIDE [`resolve_melee_strike`], so the struck piece cannot be keyed
/// by part before the verb runs. This passes the target's worn-armor resolution as a whole
/// through the verb's bare-flesh-vs-armor branch by attaching the target's FIRST worn piece (if
/// any), letting the §5 matchup + §6 wear resolve against it; a target wearing nothing folds to
/// bare flesh. The defender's effective Toughness / Luck (already projected over its injury
/// ledger) ride the [`TargetGanger`]. REUSES the verb verbatim — no combat math here.
#[expect(
    clippy::too_many_arguments,
    reason = "the fold threads the target query + the two armor relationship queries + the \
              target entity + the combatants + the projected Toughness/Luck + the weapon + \
              tuning + the three draw streams — the irreducible input set resolve_melee_strike \
              takes; bundling would only hide the access set"
)]
fn strike_with_target(
    targets: &mut MeleeTargetQuery,
    wears: &Query<&Wears>,
    pieces: &mut Query<PieceArmorMut, With<WornBy>>,
    target_entity: Entity,
    combatants: Combatants,
    toughness: Toughness,
    luck: Luck,
    weapon: MeleeWeaponHit<'_>,
    tuning: &CombatTuning,
    fight_rng: &mut FightRng,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
) -> MeleeStrike {
    // Resolve the target's struck worn piece view — the FIRST worn piece (if any), read off
    // `target → Wears → the piece entity`. The §6 fold wears its `&mut ArmorIntegrity` in
    // place; a target wearing nothing folds to bare flesh. (The §4 part roll inside the verb
    // owns the location; this resolves the target's worn protection as the verb's armor input.)
    let piece_view = wears
        .get(target_entity)
        .ok()
        .and_then(|worn| worn.pieces().next())
        .and_then(|piece_entity| {
            pieces.get_mut(piece_entity).ok().map(|piece| StruckPiece {
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
            })
        });

    let Ok((mut hp, mut wounds, mut life, mut inflicted, ..)) = targets.get_mut(target_entity)
    else {
        return MeleeStrike {
            connect:  false,
            severity: crate::severity::Severity::None,
        };
    };
    resolve_melee_strike(
        combatants,
        weapon,
        TargetGanger {
            hp: &mut hp,
            wounds: &mut wounds,
            life: &mut life,
            piece: piece_view,
            inflicted: &mut inflicted,
            toughness,
            luck,
        },
        target_entity,
        tuning,
        fight_rng,
        shot_rng,
        severity_rng,
    )
}

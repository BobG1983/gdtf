//! The `dispatch_fire` system — gate the buffered request (firing arc +
//! turn-to-fire), front-load the turn, run the landed [`fire`] verb, and bridge the
//! volley's outputs onto the boundary signals.

use bevy::{
    ecs::system::ParamSet,
    prelude::{MessageReader, Res, ResMut},
};

use super::{
    arc::{FireArcDecision, decide_fire_arc},
    emit::emit_round_signals,
    params::{BattleGridsParam, TurnQuery, WeaponProbes},
    signals::{FireDeclaration, FireSignals},
};
use crate::{
    acts::request::FireRequested,
    fire::{
        FireOrder, PieceQuery, ShooterQuery, TargetQuery, WeaponQuery, WearsQuery, WieldsQuery,
        fire,
    },
    ganger::{Aiming, Direction, Facing, Tu, TuMax},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::mode_tu_cost,
    metric::CellLevel,
    rng::{InjuryRng, SeverityRng, ShotRng},
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::DamageType,
};

/// **Dispatch** buffered [`FireRequested`] messages with the GTW-242 **firing-arc +
/// turn-to-fire** gate, then run the landed [`fire`] verb (E10.2 AC3 / GTW-242).
///
/// For each request the gate is computed from the read state BEFORE any mutation
/// ([`decide_fire_arc`]), so the spend/turn/shot is atomic-by-construction
/// (`docs/combat/resolution.md` §1; USER ruling 2026-06-16):
///
/// - **In-arc** ([`FireArcDecision::FireInArc`]): run [`fire`] directly (facing unchanged).
///   [`fire`]'s own [`crate::magazine::can_fire`] gate + [`crate::tuning::TurnTu`]-free
///   charge handle fire-TU affordability — an unaffordable in-arc shot resolves to nothing.
/// - **Out-of-arc + affordable** ([`FireArcDecision::TurnThenFire`]): ATOMICALLY spend the
///   turn TU + set the new [`Facing`] (the [`TurnQuery`] half of the [`ParamSet`]) THEN run
///   [`fire`] (the [`ShooterQuery`] half), which spends the fire TU and resolves the shot.
///   The combined gate guarantees the remaining pool still affords [`fire`]'s charge.
/// - **Out-of-arc + unaffordable** ([`FireArcDecision::Reject`]): no TU spent, no facing
///   change, no shot — `continue`.
///
/// The turn-write query ([`TurnQuery`]) and [`ShooterQuery`] both touch `Facing`/`Tu`, so
/// they are time-multiplexed through a [`ParamSet`] (`bevy-traps.md` #3 / #7 — no
/// `&mut World`); the turn write is taken FIRST, the [`fire`] re-borrow SECOND. No act
/// logic is reimplemented — the shot resolution REUSES [`fire`] verbatim; this slice only
/// GATES it and front-loads the turn.
///
/// **The GTW-290 / GTW-302 fire signal.** AFTER the volley resolves (before it is dropped)
/// this emits ONE [`ShotFired`](crate::shot_fired::ShotFired) message per ROUND fired, zipping the parallel
/// [`Volley::shots`](crate::fire::Volley::shots) geometry with the
/// [`Volley::reports`](crate::fire::Volley::reports) verdicts so each message carries BOTH
/// that round's already-computed [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) AND its
/// [`HitReport`](crate::resolve_and_apply::HitReport) ([`ShotFired::from_round`](crate::shot_fired::ShotFired::from_round)) — so the
/// presenter draws a muzzle / tracer / impact FX per round (a burst → multiple tracers) AND
/// the floating-combat-text presenter (GTW-302) draws that round's damage / wound / severity
/// / armor verdict. The two vectors are parallel (`reports[i]`/`shots[i]` are the same fired
/// round), so the report rides `Some` for every fired round. This changes NO fire-result
/// logic — it only EXPOSES the trajectory + report the volley already computed (the
/// [`MessageWriter`](bevy::prelude::MessageWriter), `bevy-traps.md` #4 / #7).
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-323 armor + weapon relationships add the disjoint wears/pieces + \
              wields/weapons system params to the dispatch_fire signature; each is a \
              distinct, independently-borrowed Bevy SystemParam that cannot be bundled \
              without a custom SystemParam struct that would only obscure the access set"
)]
pub fn dispatch_fire(
    mut requests: MessageReader<FireRequested>,
    mut shooter_set: ParamSet<(ShooterQuery, TurnQuery)>,
    mut targets: TargetQuery,
    // GTW-323 / ADR-0004: the disjoint worn-armor relationship queries `fire()` resolves
    // a struck piece through (`ganger → Wears → the BodyPart-tagged piece`). `wears` reads
    // `&Wears` on gangers (a different component than `targets`' set); `pieces` reads+wears
    // the piece entities (a different entity set) — so neither conflicts with the
    // ShooterQuery/TurnQuery/TargetQuery access (no ParamSet needed).
    wears: WearsQuery,
    mut pieces: PieceQuery,
    // GTW-323 slice 2 / ADR-0004: the disjoint wielded-weapon relationship queries `fire()`
    // resolves the shooter's weapon through (`ganger → Wields → the weapon entity`).
    // `wields` reads `&Wields` on gangers (a different component than the shooter set);
    // `weapons` reads the weapon stats + decrements the `Magazine` on the weapon entities
    // (a different entity set) — so neither conflicts with the shooter/turn/target access.
    wields: WieldsQuery,
    mut weapons: WeaponQuery,
    // GTW-505 C5 + GTW-543: the two wielded-weapon marker probes, grouped (WeaponProbes) so the
    // system stays under Bevy's 16-param limit. `melee` (GTW-505) EXCLUDES the ganger's melee
    // weapon from the ranged resolution; `mounted` (GTW-543) PREFERS the emplacement's bolted-down
    // gun the manning ganger fires. Both cheap unit-item probes over the weapon entities, disjoint
    // from `weapons` (which filters `With<WieldedBy>` and reads the stat columns), so no ParamSet.
    probes: WeaponProbes,
    mut grids: BattleGridsParam,
    tuning: Res<CombatTuning>,
    // GTW-14: disjoint per-subsystem RNG resources. ShotRng drives cone-sample +
    // body-part-roll; SeverityRng drives the §6 severity term. Two distinct
    // ResMut<T> are disjoint Bevy params (different resource types), so the
    // scheduler can parallelize this system against systems on other streams.
    // No system may take Res<ShotRng> or Res<SeverityRng> — see rng::streams doc.
    mut shot_rng: ResMut<ShotRng>,
    mut severity_rng: ResMut<SeverityRng>,
    // GTW-438: the injury-roll inputs threaded into `fire()`. `InjuryTables` /
    // `InjuryRegistry` are read (the weighted-pick table + the name→def resolution); they
    // are app/Load-OWNED resources (NOT inserted by the sim's `setup_battle`, unlike the
    // RNG streams), so a sim-only headless harness that opens a battle WITHOUT the Load
    // flow has neither — hence `Option<Res<…>>` + an empty-default fallback (bevy-traps.md
    // #1: a missing battle-lifetime resource must not panic a runtime system). With them
    // absent the roll finds no bucket and inflicts no injury, but STILL takes its one
    // InjuryRng draw (content-independent stream alignment). `InjuryRng` itself IS sim-set
    // (inserted by `setup_battle` alongside the other four streams), so it is a required
    // `ResMut`.
    injury_tables: Option<Res<InjuryTables>>,
    injury_registry: Option<Res<InjuryRegistry>>,
    mut injury_rng: ResMut<InjuryRng>,
    mut signals: FireSignals,
) {
    // Empty fallbacks for an asset-less harness (no Load flow → no InjuryTables/Registry).
    // A `Res` derefs to `&T`; an absent one falls back to a freshly-built empty default,
    // so `fire()` always gets a valid `&InjuryTables` / `&InjuryRegistry` to roll against
    // (the roll then finds no bucket but still takes its one draw).
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = injury_tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = injury_registry.as_deref().unwrap_or(&empty_registry);
    for request in requests.read() {
        // (1) READ the arc-relevant shooter state through the ShooterQuery half, copying
        //     every Copy value out so the query borrow ends at the block boundary (freeing
        //     the ParamSet to lend p1 below). A shooter not in the query (despawned) fires
        //     nothing (fail-closed).
        let shooters = shooter_set.p0();
        // The trailing `_` ignores the GTW-526 `Option<&Suppressed>` group member — the
        // arc-check read needs only pos/facing/aiming/tu_max; suppression enters the shot
        // math through the composer (`stability_for`), not this dispatch-arc gate.
        let Ok(((position, facing, _, aiming, _, _, tu_max, _), _, tu)) =
            shooters.get(request.shooter)
        else {
            continue;
        };
        // The canonical CellLevel::cell accessor through Position's deref (GTW-565).
        let actor_cell = position.cell();
        let facing: Direction = **facing;
        let tu: Tu = *tu;
        let tu_max: TuMax = *tu_max;
        let aiming: Aiming = *aiming;

        // GTW-323 slice 2: the weapon's DamageType (GTW-306) now lives on the related
        // weapon entity (`ganger → Wields → the weapon entity`), read here so the
        // per-round ShotFired can carry it (pure exposure; no fire-result change). A
        // shooter wielding no weapon — or whose weapon entity is not in the weapon query
        // — fires nothing (fail-closed, the same outcome `fire()` reaches internally).
        // GTW-505 C5: resolve the RANGED weapon (excluding the melee weapon the ganger
        // also wields) so the ShotFired carries the GUN's DamageType, never the melee
        // weapon's — the same ranged-filtered resolution `fire()` does internally.
        // GTW-543: PREFER the emplacement's mounted gun (the ganger is manning it) over its own
        // carried gun, so the ShotFired carries the MOUNTED gun's DamageType while occupied — the
        // same mounted-preferring resolution `fire()` does internally.
        let Some(weapon_entity) = wields.get(request.shooter).ok().and_then(|w| {
            w.mounted_weapon(|entity| probes.mounted.get(entity).is_ok())
                .or_else(|| w.ranged_weapon(|entity| probes.melee.get(entity).is_ok()))
        }) else {
            continue;
        };
        // The WeaponQuery row is (base_spread, accuracy, kickback, fatal_bias, damage,
        // punch, shred, DAMAGE_TYPE, stable, magazine) — the 8th leaf is the DamageType.
        let Ok((_, _, _, _, _, _, _, damage_type, ..)) = weapons.get(weapon_entity) else {
            continue;
        };
        let damage: DamageType = *damage_type;

        // (2) The fire-TU cost — the EXISTING fire-act charge (mode_tu_cost), the same
        //     source fire()'s own debit reads; both gates agree on the cost.
        let fire_cost = mode_tu_cost(&request.mode, &tu_max, &aiming, &tuning);

        // (3) The arc verdict, computed BEFORE any mutation (atomic-by-construction).
        match decide_fire_arc(
            facing,
            actor_cell,
            request.target_cell,
            tu,
            fire_cost,
            &tuning,
        ) {
            FireArcDecision::Reject => continue, // unaffordable turn+fire: no spend, no shot
            FireArcDecision::TurnThenFire {
                facing: target_facing,
                turn_cost,
            } => {
                // ATOMICALLY front-load the turn through the TurnQuery half: set the new
                // facing + spend the turn TU. The combined gate guarantees the remaining
                // pool still affords fire()'s charge below.
                let mut turners = shooter_set.p1();
                let Ok((mut actor_facing, mut actor_tu)) = turners.get_mut(request.shooter) else {
                    continue;
                };
                *actor_facing = Facing::new(target_facing);
                spend_tu(&mut actor_tu, turn_cost);
                // `turners` (the p1 borrow) ends with this match arm's block, freeing the
                // ParamSet to re-lend p0 for fire() below — no explicit drop needed.
            }
            FireArcDecision::FireInArc => {} // direct shot: no turn, fall through to fire()
        }

        // (3b) GTW-328: declare the shot for the combat-text LOG — ONCE per fire request
        //      that proceeds (the Reject arm `continue`d above, so a rejected/unaffordable
        //      shot logs nothing). Emitted BEFORE the shot rolls, carrying ONLY data the
        //      dispatch already holds: the shooter ref, the resolved target occupant at the
        //      aimed (cell, level) (a single O(1) occupancy peek — NOT a fresh raycast or
        //      re-resolve), and the request's mode kind. No RNG draw, no fire-result logic
        //      — the determinism property is untouched.
        let aim_cell_level = CellLevel::new(request.target_cell, request.target_level);
        let target = grids.occupant_at(aim_cell_level);
        signals.declarations.write(FireDeclaration::new(
            request.shooter,
            target,
            request.mode.kind,
        ));

        // (4) Run the landed verb ONCE (REUSED verbatim) — it spends the fire TU and
        //     resolves the shot. The volley's effects are the in-world mutations
        //     (TU / ammo / target surfaces) the presenter observes via change-detection.
        let order = FireOrder {
            mode:         &request.mode,
            target_cell:  request.target_cell,
            target_level: request.target_level,
        };
        let mut shooters = shooter_set.p0();
        let volley = fire(
            request.shooter,
            order,
            &mut shooters,
            &mut targets,
            &wears,
            &mut pieces,
            &wields,
            &mut weapons,
            &probes.melee,
            &probes.mounted,
            grids.grids(),
            &tuning,
            &mut shot_rng,
            &mut severity_rng,
            tables,
            registry,
            &mut injury_rng,
        );

        // (5) Emit the per-round output signals — the ShotFired FX/FCT, the injury bridge
        //     (GTW-438), and the structural cover/slab/ground bridges (GTW-364/365/366) —
        //     all PURE EXPOSURE of the volley the fire already produced (no recompute, no
        //     extra draw). Extracted to keep dispatch_fire under clippy's line gate.
        emit_round_signals(request.shooter, damage, &volley, &mut signals);

        // (6) GTW-525 C3: the RANGED weapon-tag auto-shove. If the firing weapon carries the
        //     `shove` tag AND a round CONNECTED with a ganger, knock that target back one cell
        //     (in addition to the shot's damage above). Write ONE internal ShoveRequested
        //     (ShoveSource::Weapon) for the FIRST connecting-ganger round — the connect already
        //     gated + the fire TU was charged, so dispatch_shove resolves it un-gated / TU-free;
        //     it is ordered `.after(dispatch_fire)`, so this same-frame message is consumed this
        //     tick. The probe is the EXHAUSTIVE verdict read (GTW-573 C4 —
        //     `HitVerdict::struck_ganger`, a compile error for an unbridged new kind): a MISS
        //     shoves nothing, a non-`shove` weapon shoves nothing, and a round whose fold DID
        //     NOTHING (corpse-skip / defensive no-effect) shoves nothing — only a verdict that
        //     actually landed on a live ganger counts as a connect. One shove per fire act (a
        //     burst does not multiply the knock-back).
        let weapon_shoves = signals.shove_tags.get(weapon_entity).is_ok_and(|tag| **tag);
        let struck_ganger = volley
            .reports
            .iter()
            .find_map(|report| report.verdict.struck_ganger());
        if let (true, Some(struck)) = (weapon_shoves, struck_ganger) {
            signals
                .shoves
                .write(crate::acts::request::ShoveRequested::new_weapon(
                    request.shooter,
                    struck,
                ));
        }
    }
}

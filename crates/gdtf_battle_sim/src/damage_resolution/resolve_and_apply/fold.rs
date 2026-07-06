//! The E3.9 fold act — [`resolve_and_apply`], the sim's ONE **logic-free delegation
//! dispatch** over what the round struck (GTW-573 C3): each arm hands its inputs to
//! that kind's [`kinds`](super::kinds) module, which owns the whole fold + verdict
//! construction. No damage / severity / depletion logic lives here.

use bevy::prelude::Entity;

use crate::{
    ganger::Luck,
    injuries::{InjuryRegistry, InjuryTables},
    metric::CellLevel,
    resolve_and_apply::{
        kinds,
        report::{HitReport, HitVerdict, StruckSurfaces, TargetGanger},
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    rng::{InjuryRng, SeverityRng},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// Fold **one [`ShotOutcome`]** through damage → severity → application into ONE
/// frozen [`HitReport`] — the E3.9 capstone integrator (`docs/combat/resolution.md`
/// §5 / §6 / §3), and the sim's ONE per-kind delegation dispatch (GTW-573 C3).
///
/// Statically dispatches on what the round struck ([`ShotOutcome::kind`]) — one arm
/// per struck kind, each a LOGIC-FREE handoff to that kind's `kinds`
/// module (adding a struck kind = one sibling module + one
/// [`HitVerdict`](super::report::HitVerdict) variant + one arm here):
///
/// - **[`ShotKind::Ganger`]** → `kinds::ganger::fold` — the §5/§6/§8 wound path
///   (armored-vs-bare-flesh resolution, the shared `synthesize_wound` core, the
///   GTW-544 DOT decision). The ONLY arm that draws RNG.
/// - **[`ShotKind::Cover`]** → `kinds::cover::fold` — the GTW-364 cover-hit path
///   (the SAME damage formula against the cover's own armor,
///   `CoverLedger::deplete_cover`, the destroyed-cell verdict). RNG-free.
/// - **[`ShotKind::Slab`]** → `kinds::slab::fold` — the GTW-365 slab mirror
///   (`SlabLedger::deplete_slab`). RNG-free.
/// - **[`ShotKind::Ground`]** → `kinds::ground::fold` — the GTW-366
///   damaged-never-destroyed accrual verdict. Mutates nothing, RNG-free.
/// - **[`ShotKind::Miss`]** → [`HitVerdict::NoEffect`] (no draw, no mutation).
///
/// `target` is `Some` only when the struck object is a queryable target ganger; the
/// non-ganger arms carry `None`, and a `Ganger` outcome whose entity is not a
/// queryable target folds defensively to no-effect inside the ganger module.
/// `surfaces` bundles the two model HP ledgers the structural arms spend — exactly
/// one is touched per hit (its `ShotKind` selects it). ECS queries never enter this
/// fold: the call boundary (`fire`'s composer) resolves the borrowed views (GTW-323
/// query disjointness stays at the boundary — never bundled into a trait).
///
/// Pure, render-free, SYNCHRONOUS model logic (no deferred commands-ext — the
/// mutation and the verdict land before this returns). The severity-gated draw
/// discipline is the load-bearing seeded-replay contract, owned by the ganger arm and
/// pinned by `test::draw_discipline`: a live ganger hit takes exactly ONE
/// [`SeverityRng`] draw and — on a `Minor`/`Major`/`Critical` severity ONLY — exactly
/// ONE [`InjuryRng`] draw (empty-table draw-then-discard included); a graze / Fatal
/// takes no injury draw; corpse-skip and the defensive folds take neither; the
/// cover / slab / ground / miss arms take zero draws on both streams. Same
/// [`BattleSeed`](crate::rng::BattleSeed) → identical report for identical inputs.
/// Charging TU and looping the burst is the E4 `fire()` act — **out of scope** here.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-365 bundles the two structural HP ledgers (cover + slab) into the \
              StruckSurfaces param, and GTW-438 threads the injury-roll inputs (the \
              InjuryTables + InjuryRegistry reads + the &mut InjuryRng draw stream) onto \
              the wound path; the remaining args are the irreducible \
              outcome / weapon / luck / target / entity / surfaces / tuning / severity-rng \
              / injury-tables / injury-registry / injury-rng set; the target ganger \
              surfaces are ALREADY grouped in the TargetGanger bundle"
)]
pub fn resolve_and_apply(
    outcome: &ShotOutcome,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
    target: Option<TargetGanger<'_>>,
    target_entity: Entity,
    surfaces: StruckSurfaces<'_>,
    tuning: &CombatTuning,
    rng: &mut SeverityRng,
    // GTW-438: the injury-roll inputs, threaded onto the wound path. The cover / slab /
    // ground / miss arms ignore them (a structural hit rolls no injury and takes no
    // InjuryRng draw); only the ganger module consults them.
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    // The ONE dispatch: match the struck kind, delegate the whole fold to that kind's
    // module, and pair the returned verdict with the geometric kind. NO fold logic here.
    let verdict = match outcome.kind {
        ShotKind::Ganger(_) => kinds::ganger::fold(
            outcome,
            weapon,
            shooter_luck,
            target,
            target_entity,
            tuning,
            rng,
            tables,
            registry,
            injury_rng,
        ),
        ShotKind::Cover(entry) => kinds::cover::fold(
            &entry,
            CellLevel::new(outcome.cell, outcome.level),
            weapon,
            surfaces.cover,
            tuning,
        ),
        ShotKind::Slab(at) => kinds::slab::fold(at, weapon, surfaces.slab, tuning),
        ShotKind::Ground(at) => kinds::ground::fold(at, weapon),
        ShotKind::Miss => HitVerdict::NoEffect,
    };
    HitReport {
        kind: outcome.kind,
        verdict,
    }
}

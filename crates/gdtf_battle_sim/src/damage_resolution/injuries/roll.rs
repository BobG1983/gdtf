//! The in-fold injury **roll** — [`roll_injury`], the GTW-438 first (and only) draw
//! site on the [`InjuryRng`] stream.
//!
//! When a non-graze, non-fatal wound lands on a ganger, the damage fold calls
//! [`roll_injury`] to pick a named injury from the weighted `(body_part, severity)`
//! table and freeze the verdict onto the [`HitReport`](crate::resolve_and_apply::HitReport).
//!
//! ## Draw discipline (the GTW-405 §"Determinism / replay" contract)
//!
//! - A [`Severity::None`] (graze) and a
//!   [`Severity::Fatal`] (death via the existing
//!   terminal gate) are **never tabled** — [`roll_injury`] returns
//!   `None` with **no draw and no side effect**.
//! - A [`Severity::Minor`] /
//!   [`Severity::Major`] /
//!   [`Severity::Critical`] wound **always takes EXACTLY
//!   ONE** [`InjuryRng`] sample — even when the `(part, severity)` table is
//!   empty/missing (then the sample is drawn-then-discarded and a
//!   [`warn_once!`](bevy::log::warn_once) fires). This makes the stream
//!   **content-independent**: the [`InjuryRng`] cursor advances identically whether or
//!   not content exists for that bucket, so a content hot-edit can never desync replay
//!   within a fixed severity outcome.

use bevy::log::warn_once;

use super::{InjuryTables, RolledInjury, WeightedInjuryTable};
use crate::{armor::BodyPart, injuries::InjuryRegistry, rng::InjuryRng, severity::Severity};

/// Roll a named [`RolledInjury`] for a non-graze, non-fatal `severity` wound to `part`,
/// resolving the picked key against the [`InjuryRegistry`] (`docs/combat/resolution.md`
/// injury tables; GTW-405 / GTW-438).
///
/// The single [`InjuryRng`] draw site. The draw discipline (the GTW-405 §"Determinism /
/// replay" contract — see the module docs):
///
/// - [`Severity::None`] / [`Severity::Fatal`] are NOT tabled → returns
///   `None` with **no draw** and no side effect.
/// - [`Severity::Minor`] / [`Severity::Major`] / [`Severity::Critical`] **always draw
///   EXACTLY ONE** [`InjuryRng`] sample (a cumulative-weight pick over the
///   canonically-sorted bucket, the [`roll_body_part`](crate::hit_location::roll_body_part)
///   precedent). An **empty / missing** bucket STILL draws one sample (then discards it
///   + [`warn_once!`](bevy::log::warn_once)) so the stream stays content-independent.
///
/// Returns `Some` the rolled injury (a snapshot of the picked
/// [`InjuryDef`](super::InjuryDef)'s name / severity / frozen effects / three texts,
/// stamped with the STRUCK `part` — NOT the def's side-agnostic `body_part`, so a
/// shared-pool `DisableHand` disables the hand on the struck side, GTW-440 C3) when the
/// bucket has content AND the picked key resolves in `registry`;
/// `None` when the severity is not tabled, the bucket is empty/missing,
/// or the picked key is unknown (each of the latter two STILL having taken the one draw).
/// Pure given the [`InjuryRng`] state — the same cursor position yields the same pick.
#[must_use]
pub fn roll_injury(
    part: BodyPart,
    severity: Severity,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    rng: &mut InjuryRng,
) -> Option<RolledInjury> {
    // (1) Only the three tabled severities roll — None (graze) and Fatal (death via the
    //     existing terminal gate) take NO draw and have NO side effect (the §8 tabling
    //     rule). Returning before the draw is what keeps a graze / fatal off the stream.
    match severity {
        Severity::None | Severity::Fatal => return None,
        Severity::Minor | Severity::Major | Severity::Critical => {}
    }

    // (2) The bucket for this (part, severity). It may be absent (no weighting authored
    //     the bucket) or empty (authored but every key dropped at build) — in EITHER case
    //     we still take the one draw below, for content-independent stream alignment.
    let table = tables.table(part, severity);

    // (3) THE ONE DRAW — taken UNCONDITIONALLY for a tabled severity, before any content
    //     check, so the InjuryRng cursor advances identically whether the bucket has
    //     content or is empty/missing (the §9 content-independence property). A populated
    //     bucket maps the draw onto its rows by cumulative weight (the roll_body_part
    //     precedent); an empty/missing bucket discards the sample and warns.
    let pick = pick_key(table, rng);

    let Some(picked) = pick else {
        // Empty / missing bucket — the draw was still taken (above) and is discarded. A
        // tabled severity with no rollable content can never inflict an injury; warn once
        // (per (part, severity) message) rather than spam every hit.
        warn_once!(
            "injury roll for ({part:?}, {severity:?}) found no rollable content \
             (empty/missing weighting bucket); the InjuryRng draw was taken and discarded"
        );
        return None;
    };

    // (4) Resolve the picked key against the registry → the authored InjuryDef. An
    //     unknown key (the build-time audit WARNs on these, GTW-437) yields no injury —
    //     the draw was already taken, so the stream alignment holds.
    let Some(def) = registry.def(&picked) else {
        warn_once!(
            "injury roll picked key `{}` for ({part:?}, {severity:?}) but it is not in \
             the InjuryRegistry; no injury inflicted (the draw was taken)",
            &*picked
        );
        return None;
    };

    // (5) Freeze the verdict — a snapshot of the def's applied state (name / severity /
    //     the frozen effects + the three routed texts) for HitReport.injury, stamped with
    //     the STRUCK `part`, NOT the def's `body_part`. Since GTW-440 the per-side parts
    //     share one CATEGORY pool (a single `shattered_hand` is rolled for either arm), so
    //     the struck side is the authoritative location — this is what makes a
    //     `DisableHand` rolled on `RightArm` disable the RIGHT hand even though the def is
    //     side-agnostic (the side-from-part design, GTW-443 / GTW-440 C3). The def's own
    //     `body_part` only routes which category POOL it was authored into.
    Some(RolledInjury::new(
        def.name.clone(),
        part,
        def.severity,
        def.effects.clone(),
        def.popup_text.clone(),
        def.log_text.clone(),
        def.inspect_text.clone(),
    ))
}

/// Take the ONE [`InjuryRng`] draw and map it onto the bucket's rows by cumulative
/// weight, returning the picked [`InjuryName`](super::InjuryName) key — or
/// [`None`](Option::None) when the bucket is empty/missing or has all-zero weights
/// (the draw is **always taken** regardless, for stream alignment).
///
/// The cumulative-weight pick is the [`roll_body_part`](crate::hit_location::roll_body_part)
/// precedent: sum the rows' weights, draw once in `0..total`, and return the row whose
/// running cumulative first exceeds the draw. The bucket's rows are already canonically
/// sorted by the loader (GTW-437), so the pick is folder-enumeration-order-independent.
///
/// CRITICAL — the draw is taken FIRST, unconditionally, so even an empty/missing/all-zero
/// bucket advances the [`InjuryRng`] cursor by exactly one sample (then this returns
/// [`None`](Option::None)). A real `random_range(0..=upper)` is taken on a non-empty
/// bucket; an empty/all-zero bucket takes an equivalent single `random_range(0..=0)` so
/// the cursor still advances exactly once.
fn pick_key(table: Option<&WeightedInjuryTable>, rng: &mut InjuryRng) -> Option<super::InjuryName> {
    // Sum the bucket's weights as u64 so many rows cannot overflow the total (the rows
    // are u32 weights; an empty/missing bucket sums to 0). A missing bucket is an empty
    // slice (the `WeightedInjuryTable` derefs to its rows).
    let rows: &[super::WeightedInjuryEntry] = table.map_or(&[], |t| t);
    let total: u64 = rows.iter().map(|row| u64::from(*row.weight)).sum();

    // THE ONE DRAW — taken unconditionally. For an empty/missing/all-zero bucket the
    // range collapses to `0..=0` (a single fixed sample) so the cursor still advances by
    // exactly one; for a populated bucket it is `0..=(total-1)`.
    let upper = total.saturating_sub(1);
    let draw: u64 = rng.random_range(0..=upper);

    // No rollable content (no rows, or every weight 0 → total 0) — the draw was taken
    // above; report the empty pick.
    if total == 0 {
        return None;
    }

    // Map the draw onto the rows by cumulative weight: the row whose running cumulative
    // first exceeds the draw is the pick. A zero-weight row adds nothing, so its half-open
    // slice is empty and it is never selected.
    let mut cumulative: u64 = 0;
    for row in rows {
        cumulative += u64::from(*row.weight);
        if draw < cumulative {
            return Some(row.injury.clone());
        }
    }

    // Unreachable in practice (draw < total, the cumulative reaches total) — but stay
    // panic-free: report no pick rather than unwrap.
    None
}

//! The E3.9 fold act — [`resolve_and_apply`] composes matchup → [`resolve_hit`] →
//! [`roll_severity`] → [`apply_hit`] into ONE model-side act, plus the
//! [`struck_piece`] armored-vs-bare-flesh resolution it runs against.

use bevy::prelude::Entity;

use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType},
    armor_wear::ArmorWearOutcome,
    cover::{CoverDamage, CoverEntry, CoverEvent, CoverLedger},
    ganger::{LifeState, Luck},
    injuries::{InjuryRegistry, InjuryTables, roll_injury},
    matchup::{Matchup, matchup},
    metric::{Cell, CellLevel},
    resolve_and_apply::report::{
        AppliedDamage, GroundAccrual, HitReport, StruckPiece, StruckSurfaces, TargetGanger,
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    resolve_hit::resolve_hit,
    rng::{InjuryRng, SeverityRng},
    severity::{SeverityInputs, part_severity_mod, roll_severity},
    slab::{SlabDamage, SlabEntry, SlabEvent, SlabLedger},
    surface::GroundDamage,
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// The **bare-flesh** armor piece — a zeroed soak used when the struck location no
/// longer protects (its piece entity's [`ArmorIntegrity`](crate::armor::ArmorIntegrity)
/// `≤ 0`; `weapons-and-armor.md` §"Per-hit resolution": "later hits on that location
/// resolve as bare flesh").
///
/// Floor / protection / hardness are all `0` so the per-hit formula soaks nothing
/// (on bare flesh `dmg == damage` regardless of matchup, since protection `0`),
/// integrity is `0` (the piece is already worn through — there is nothing to wear),
/// and the type is [`ArmorType::DEFAULT`] (an explicit placeholder — the matchup on
/// bare flesh is forced to [`Matchup::Neutral`], so the armor type is never matched
/// against). A `const` since every stat is a constant zero.
const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// Resolve the struck location to the `(`[`ArmorPiece`]`, `[`Matchup`]`)` the
/// per-hit formula runs against — the armored branch or **bare flesh**.
///
/// If a worn [`StruckPiece`] is present AND still [`protects`](StruckPiece::protects),
/// returns that piece's stats (read off its piece-entity components — GTW-323 /
/// ADR-0004) and the E3.2 [`matchup`] of the weapon's
/// [`DamageType`](crate::weapon::DamageType) vs the piece's [`ArmorType`] (the wheel
/// advantage applies). Otherwise (no piece at this location, or it is worn through)
/// returns the zeroed [`BARE_FLESH`] piece under [`Matchup::Neutral`] — there is no
/// armor type to match against, so no wheel advantage, and the zeroed soak means the
/// hit lands as full weapon damage (`weapons-and-armor.md` §"Per-hit resolution").
fn struck_piece(piece: Option<&StruckPiece<'_>>, weapon: WeaponStats<'_>) -> (ArmorPiece, Matchup) {
    match piece {
        Some(p) if p.protects() => {
            // Re-assemble the read-only ArmorPiece value the damage formula consumes
            // from the piece entity's stat components (integrity is read by value here;
            // the wear mutation happens later in `apply_hit`).
            let assembled = ArmorPiece::new(
                p.floor,
                p.protection,
                p.integrity_value(),
                p.hardness,
                p.armor_type,
            );
            let resolved = matchup(*weapon.damage_type, p.armor_type);
            (assembled, resolved)
        }
        // No protecting piece (missing piece OR worn through) ⇒ bare flesh: no
        // protection / hardness, no armor type to match → Neutral.
        _ => (BARE_FLESH, Matchup::Neutral),
    }
}

/// The bare-flesh-shaped [`ArmorPiece`] a piece of cover's hit resolves against —
/// the cover's own [`ArmorProtection`] / [`ArmorHardness`] (its `floor` 0 — cover
/// has no "still bruises" floor; its `integrity` 0 — the formula never reads it; its
/// type [`ArmorType::DEFAULT`] — cover has no matchup-wheel node, so the matchup is
/// forced [`Matchup::Neutral`] below).
///
/// Cover uses the **same armor/damage model as a ganger** (`docs/combat/resolution.md`
/// §3), so the cover hit runs the EXACT same [`resolve_hit`] formula the ganger path
/// runs — only the armor stats it resolves against come from the struck
/// [`CoverEntry`] instead of a worn piece.
const fn cover_armor_piece(entry: &CoverEntry) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(0),
        entry.armor_protection,
        ArmorIntegrity::new(0),
        entry.armor_hardness,
        ArmorType::DEFAULT,
    )
}

/// Spend a shot's damage against the struck cover and return the [`CoverEvent`]
/// outcome — the `apply_cover_hit` bridge (`docs/combat/resolution.md` §3: cover
/// "uses the same armor/damage model as a ganger … `apply_cover_hit` spends [HP], and
/// depletion emits a cover-destroyed event").
///
/// The pipeline (C1 → C2):
///
/// 1. **Damage (C1)** — the SAME [`resolve_hit`] formula the ganger path uses, run
///    against the cover's own armor stats ([`cover_armor_piece`]) under
///    [`Matchup::Neutral`] (cover has no wheel node). Its [`HpDamage`](crate::resolve_hit::HpDamage)
///    is the HP the hit removes — reused verbatim, NOT a new parallel formula.
/// 2. **Deplete (C2)** — that HP, converted to a [`CoverDamage`] (clamped at zero —
///    a fully-soaked hit removes no HP), is spent via the EXISTING
///    [`CoverLedger::deplete_cover`]. The prototype is the struck `entry` (so a
///    never-before-hit piece lazy-seeds at its authored `max_hp`); HP bookkeeping and
///    destruction detection are owned by `deplete_cover`, never re-implemented here.
///
/// Returns the [`CoverEvent`] (`Damaged` / `Destroyed`) — the caller bridges a
/// `Destroyed` into the buffered message (C3). The `at` `(cell, level)` is the
/// struck outcome's cell/level (the cover the round stopped on).
fn apply_cover_hit(
    entry: &CoverEntry,
    at: CellLevel,
    weapon: WeaponStats<'_>,
    cover: &mut CoverLedger,
    tuning: &CombatTuning,
) -> CoverEvent {
    // (1) C1 — the per-hit damage formula, REUSED verbatim (the ganger path's E3.3),
    //     against the cover's own armor stats under Neutral (cover has no wheel node).
    let piece = cover_armor_piece(entry);
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    // The resolved HP-loss damage → a CoverDamage. HpDamage is a signed i32 (it can be
    // negative pre-floor, though floor 0 keeps it ≥ 0 here); a fully-soaked hit removes
    // no HP, so clamp the conversion at zero (cover HP is a non-negative pool).
    let removed = CoverDamage::new(u32::try_from((*hit.hp_damage).max(0)).unwrap_or(0));

    // (2) C2 — spend it through the EXISTING ledger API (HP bookkeeping + destruction
    //     detection owned there). The prototype is the struck entry, so a never-hit
    //     piece lazy-seeds at its authored max_hp before this hit deducts.
    cover.deplete_cover(at, removed, *entry, tuning)
}

/// The bare-flesh-shaped [`ArmorPiece`] a floor/roof slab's hit resolves against — the
/// slab's own [`ArmorProtection`] / [`ArmorHardness`] (its `floor` 0 / `integrity` 0 /
/// type [`ArmorType::DEFAULT`] forced [`Matchup::Neutral`], the same shape
/// [`cover_armor_piece`] builds for cover).
///
/// A slab uses the **same armor/damage model as a ganger and cover**
/// (`docs/combat/resolution.md` §3.1; user-ruled 2026-06-22), so the slab hit runs the
/// EXACT same [`resolve_hit`] formula the ganger / cover path runs — only the armor
/// stats it resolves against come from the struck [`SlabEntry`].
const fn slab_armor_piece(entry: &SlabEntry) -> ArmorPiece {
    ArmorPiece::new(
        ArmorFloor::new(0),
        entry.armor_protection,
        ArmorIntegrity::new(0),
        entry.armor_hardness,
        ArmorType::DEFAULT,
    )
}

/// Spend a shot's damage against the struck slab and return the [`SlabEvent`] outcome
/// — the `apply_slab_hit` bridge (the slab mirror of [`apply_cover_hit`];
/// `docs/combat/resolution.md` §3.1, user-ruled 2026-06-22).
///
/// The pipeline (C2):
///
/// 1. **Damage** — the SAME [`resolve_hit`] formula the ganger / cover path uses, run
///    against the slab's own armor stats ([`slab_armor_piece`]) under
///    [`Matchup::Neutral`] (a slab has no wheel node). Its
///    [`HpDamage`](crate::resolve_hit::HpDamage) is the HP the hit removes — reused
///    verbatim, NOT a new parallel formula.
/// 2. **Deplete** — that HP, converted to a [`SlabDamage`] (clamped at zero — a
///    fully-soaked hit removes no HP), is spent via the EXISTING
///    [`SlabLedger::deplete_slab`]. The prototype is the struck `entry` (so a
///    never-before-hit slab lazy-seeds at its [`SlabDefaults`](crate::tuning::SlabDefaults)
///    `max_hp`); HP bookkeeping + destruction detection are owned by `deplete_slab`,
///    never re-implemented here.
///
/// Returns the [`SlabEvent`] (`Damaged` / `Destroyed`) — the caller bridges a
/// `Destroyed` into the buffered message (C3). Takes **no** RNG draw — the slab-vs-armor
/// formula is deterministic (replay-safe). The `at` `(cell, level)` is the struck
/// outcome's surface cell (the slab the round stopped on).
fn apply_slab_hit(
    entry: &SlabEntry,
    at: CellLevel,
    weapon: WeaponStats<'_>,
    slab: &mut SlabLedger,
    tuning: &CombatTuning,
) -> SlabEvent {
    // (1) The per-hit damage formula, REUSED verbatim (the ganger / cover path's E3.3),
    //     against the slab's own armor stats under Neutral (a slab has no wheel node).
    let piece = slab_armor_piece(entry);
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        Matchup::Neutral,
        tuning,
    );

    // The resolved HP-loss damage → a SlabDamage. HpDamage is a signed i32 (floor 0 keeps
    // it ≥ 0 here); a fully-soaked hit removes no HP, so clamp the conversion at zero
    // (slab HP is a non-negative pool).
    let removed = SlabDamage::new(u32::try_from((*hit.hp_damage).max(0)).unwrap_or(0));

    // (2) Spend it through the EXISTING ledger API (HP bookkeeping + destruction detection
    //     owned there). The prototype is the struck entry, so a never-hit slab lazy-seeds
    //     at its SlabDefaults max_hp before this hit deducts. PERSISTENT across strikes:
    //     a second hit reads the reduced current_hp from the map (C4).
    slab.deplete_slab(at, removed, *entry)
}

/// Build the **ground-accrual verdict** for a round that struck the ground at `at` —
/// the `apply_ground_hit` bridge (`docs/combat/resolution.md` §3.2; user-ruled
/// 2026-06-22; the ground-accrual mirror of [`apply_cover_hit`] / [`apply_slab_hit`]).
///
/// The ground is **damaged, never destroyed**, so there is NO armor / HP / depletion
/// math here — the accrued amount is simply the round's `weapon_damage` (GTW-366 C4,
/// NOT a constant and NOT a tuning leaf — the ground has no HP/armor defaults), recorded
/// against the ground-plane [`Cell`] the round exited through. [`WeaponDamage`](crate::weapon::WeaponDamage)
/// is a signed `i32` (it carries a `Default` spawn sentinel that can read `0`/negative
/// before the real value seeds); a negative value clamps to zero, mirroring the
/// cover / slab `u32::try_from(... .max(0))` conversion (ground damage is a non-negative
/// pool). Takes **no** RNG draw — the ground path is deterministic (replay-safe). The
/// caller bridges the returned [`GroundAccrual`] into the buffered
/// [`GroundAccrued`](crate::occupancy_sync::GroundAccrued) message, which
/// [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground) accrues
/// (monotonically) onto the [`SurfaceGrid`](crate::surface::SurfaceGrid). It mutates
/// NOTHING (no ledger, no grid, no ganger) — purely a frozen verdict.
fn apply_ground_hit(at: CellLevel, weapon: WeaponStats<'_>) -> GroundAccrual {
    // The ground-plane (x, y) the round exited through — the accumulator key (a Cell,
    // never the storey z). CellLevel derefs to its inner IVec3 (the `actor_cell` split
    // precedent).
    let cell = Cell::new(at.x, at.y);
    // C4: the accrued amount is the round's weapon_damage — NOT a hardcoded constant and
    // NOT a tuning leaf. WeaponDamage is a signed i32; a negative / sentinel value clamps
    // to zero (ground damage is a non-negative pool), the same conversion the cover / slab
    // HP-loss takes.
    let amount = GroundDamage::new(u32::try_from((**weapon.damage).max(0)).unwrap_or(0));
    GroundAccrual::new(cell, amount)
}

/// Fold **one [`ShotOutcome`]** through damage → severity → application into ONE
/// frozen [`HitReport`] — the E3.9 capstone integrator (`docs/combat/resolution.md`
/// §5 / §6 / §3).
///
/// Dispatches on what the round struck ([`ShotOutcome::kind`]):
///
/// - **[`ShotKind::Ganger`]** — the wound path: composes the already-built E3 verbs
///   (E3.2 [`matchup`] → E3.3 [`resolve_hit`] → E3.4 [`roll_severity`] → E3.6
///   [`apply_hit`]) onto the [`TargetGanger`], taking the ONE severity draw. See
///   [`fold_ganger`].
/// - **[`ShotKind::Cover`]** — the cover-hit path (GTW-364, resolution.md §3): cover
///   uses the **same armor/damage model as a ganger**, so the SAME [`resolve_hit`]
///   formula resolves the hit damage against the struck cover's own armor stats, that
///   HP is spent through the EXISTING [`CoverLedger::deplete_cover`], and a depletion
///   to zero records the destroyed `(cell, level)` in
///   [`HitReport::cover_destroyed`] (the fire path bridges it to a buffered
///   [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) message). Takes **no**
///   RNG draw — the cover-vs-armor formula is deterministic. See [`apply_cover_hit`].
/// - **[`ShotKind::Slab`]** — the slab-hit path (GTW-365, resolution.md §3.1, user-ruled
///   2026-06-22): a slab has its **own HP + armor**, so the SAME [`resolve_hit`] formula
///   resolves the hit damage against the struck slab's own armor stats (lazily seeded
///   from the [`SlabDefaults`](crate::tuning::SlabDefaults) tuning leaf — slabs carry no
///   per-piece authored HP), that HP is spent through the EXISTING
///   [`SlabLedger::deplete_slab`], and a depletion to zero records the destroyed
///   `(cell, level)` in [`HitReport::slab_destroyed`] (the fire path bridges it to a
///   buffered [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) message). Takes
///   **no** RNG draw — the slab-vs-armor formula is deterministic. See [`apply_slab_hit`].
/// - **[`ShotKind::Ground`]** — the ground-accrual path (GTW-366, resolution.md §3.2,
///   user-ruled 2026-06-22): a round that exits the bottom of the voxel column strikes the
///   ground, which is **damaged, never destroyed** — so this records the round's
///   `weapon_damage` against the struck [`Cell`] in [`HitReport::ground_accrued`] (the fire
///   path bridges it to a buffered
///   [`GroundAccrued`](crate::occupancy_sync::GroundAccrued) message that accrues
///   monotonically onto the [`SurfaceGrid`](crate::surface::SurfaceGrid)). Mutates NOTHING
///   here (no ganger / cover / slab / grid) and takes **no** RNG draw — purely cosmetic
///   bookkeeping (crater FX is a later ticket). See [`apply_ground_hit`].
/// - **miss** — a no-effect report (no draw, no mutation).
///
/// `target` is `Some` only when the struck object is a queryable target ganger; a
/// [`ShotKind::Cover`] / [`ShotKind::Slab`] / ground / miss carries `None` (there is no
/// struck ganger), and a [`ShotKind::Ganger`] whose entity is not a queryable target
/// folds defensively to no-effect. `surfaces` bundles the two model HP ledgers the
/// structural-hit paths spend ([`StruckSurfaces::cover`] on a cover hit,
/// [`StruckSurfaces::slab`] on a slab hit) — exactly one is touched per hit (its
/// `ShotKind` selects it), both untouched on a ganger / ground / miss.
///
/// Pure, render-free model logic. On a ganger hit it mutates the target ganger's
/// battle state in place and advances the injected [`SeverityRng`](crate::rng::SeverityRng) by **exactly one**
/// severity draw; on a cover / slab hit it spends the respective ledger's HP and takes
/// no draw; on a ground hit it records the accrual in the report and mutates NOTHING
/// (the accrual reaches the [`SurfaceGrid`](crate::surface::SurfaceGrid) via the fire
/// path's message bridge, never the fold); otherwise it mutates nothing and takes no
/// draw. It **owns no mutation after return**. Same [`BattleSeed`](crate::rng::BattleSeed)
/// → identical report for identical inputs (the seeded-replay property — the cover / slab /
/// ground paths are RNG-free, so they cannot perturb the stream). Charging TU and looping
/// the burst is the E4 `fire()` act — **out of scope** here.
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
    // InjuryRng draw); only `fold_ganger` consults them.
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    match outcome.kind {
        // The wound path — only a Ganger outcome can wound (the ONE severity draw is
        // taken here; a corpse / missing part folds to no-effect with no draw). A
        // defensive `None` target (the struck entity was not a queryable ganger) folds
        // to no-effect — never a panic. GTW-438: the injury roll's one InjuryRng draw is
        // taken inside `fold_ganger` AFTER apply_hit, gated on the rolled severity.
        ShotKind::Ganger(_) => match target {
            Some(target) => fold_ganger(
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
            None => HitReport::no_effect(outcome.kind),
        },
        // The cover-hit path (GTW-364): reuse the ganger damage formula against the
        // cover's own armor, spend the ledger's HP, and record a destroyed cell — the
        // fire→deplete→message bridge. No RNG draw (deterministic, replay-safe).
        ShotKind::Cover(entry) => {
            let at = CellLevel::new(outcome.cell, outcome.level);
            let event = apply_cover_hit(&entry, at, weapon, surfaces.cover, tuning);
            let cover_destroyed = match event {
                CoverEvent::Destroyed(cell) => Some(cell),
                CoverEvent::Damaged(_) => None,
            };
            HitReport {
                kind: outcome.kind,
                part: None,
                applied: None,
                cover_destroyed,
                slab_destroyed: None,
                ground_accrued: None,
                // A cover hit never wounds a ganger, so it rolls no injury (no draw).
                injury: None,
            }
        }
        // The slab-hit path (GTW-365/396): a slab has its OWN HP + armor; reuse the
        // ganger damage formula against the slab's own armor (eagerly seeded at setup
        // from the per-slab TerrainSpec via SlabLedger::insert — GTW-396 Decision C).
        // The prototype here is the NO-PANIC FALLBACK for a slab struck with no
        // authored entry (an out-of-bounds / non-authored cell). `SlabLedger::entry_seeded`
        // (`.entry(key).or_insert(seeded(prototype...))`) returns the eagerly-inserted
        // entry UNCHANGED and ignores the fallback prototype — so authored HP is
        // always honored; the fallback fires ONLY for an unauthored strike.
        // `SLAB_FALLBACK_DEFAULTS` is a code-only const (`SlabDefaults::default()`) —
        // NOT a `tuning.slab_defaults` read (that field was removed in GTW-396).
        ShotKind::Slab(cell_level) => {
            /// Code-only fallback for an unauthored slab strike (no registry entry).
            /// NOT the tuning leaf (removed in GTW-396) — a no-panic backstop only.
            const SLAB_FALLBACK_DEFAULTS: crate::tuning::SlabDefaults =
                crate::tuning::SlabDefaults::FALLBACK;
            let prototype = SlabLedger::prototype_for(cell_level, &SLAB_FALLBACK_DEFAULTS);
            let event = apply_slab_hit(&prototype, cell_level, weapon, surfaces.slab, tuning);
            let slab_destroyed = match event {
                SlabEvent::Destroyed(cell) => Some(cell),
                SlabEvent::Damaged(_) => None,
            };
            HitReport {
                kind: outcome.kind,
                part: None,
                applied: None,
                cover_destroyed: None,
                slab_destroyed,
                ground_accrued: None,
                // A slab hit never wounds a ganger, so it rolls no injury (no draw).
                injury: None,
            }
        }
        // The ground-accrual path (GTW-366): a round that exits the bottom of the voxel
        // column strikes the GROUND, which is damaged-never-destroyed — record the round's
        // weapon_damage against the struck cell in the report (bridged to a GroundAccrued
        // message by dispatch_fire, then accrued monotonically onto the SurfaceGrid). No
        // draw, and NOTHING is mutated here — purely cosmetic (the ground arm of the cover /
        // slab mirror; crater FX is a later ticket).
        ShotKind::Ground(at) => HitReport {
            kind:            outcome.kind,
            part:            None,
            applied:         None,
            cover_destroyed: None,
            slab_destroyed:  None,
            ground_accrued:  Some(apply_ground_hit(at, weapon)),
            // A ground hit never wounds a ganger, so it rolls no injury (no draw).
            injury:          None,
        },
        // A clean miss strikes nothing — no draw, no mutation, no accrual.
        ShotKind::Miss => HitReport::no_effect(outcome.kind),
    }
}

/// Fold a **[`ShotKind::Ganger`]** outcome onto the target ganger — the wound path of
/// [`resolve_and_apply`] (`docs/combat/resolution.md` §5 / §6).
///
/// Composes the E3 verbs in order, taking the ONE severity draw:
///
/// 1. **Corpse-skip — before any draw** — a target already at [`LifeState::Dead`]
///    returns a no-effect report with **no draw and no mutation**, so a corpse never
///    consumes an RNG draw and determinism is preserved.
/// 2. **Part** — the struck [`BodyPart`] rode along on the §4 part roll (drawn
///    upstream); a defensive `None` returns a no-effect report (no draw).
/// 3. **Armor / bare flesh** — [`struck_piece`] picks the armored piece + matchup or
///    the zeroed bare-flesh piece under [`Matchup::Neutral`].
/// 4. **Damage** — [`resolve_hit`] → [`HitResult`](crate::resolve_hit::HitResult).
/// 5. **Severity (the ONE draw)** — [`roll_severity`] over the assembled
///    [`SeverityInputs`], drawing from the injected [`SeverityRng`](crate::rng::SeverityRng).
/// 6. **Apply** — [`apply_hit`] folds the hit onto the target in place.
/// 7. **Freeze** — returns the [`HitReport`] of named newtypes (`cover_destroyed`
///    always `None` — a ganger hit destroys no cover).
///
/// Split out of [`resolve_and_apply`]'s kind dispatch (GTW-364) so the ganger and
/// cover paths each stay a focused fold. Mutates the target ganger's battle state in
/// place and advances the injected [`SeverityRng`](crate::rng::SeverityRng) by exactly
/// one severity draw on a real hit; owns no mutation after return.
///
/// GTW-438 — the injury roll. AFTER [`apply_hit`] (the wound's Wounds already spent),
/// gated on the rolled [`Severity`](crate::severity::Severity), it calls
/// [`roll_injury`] over the injected [`InjuryTables`] / [`InjuryRegistry`] / `&mut`
/// [`InjuryRng`](crate::rng::InjuryRng): a [`None`](crate::severity::Severity::None)
/// (graze) / [`Fatal`](crate::severity::Severity::Fatal) takes NO injury draw; a
/// `Minor`/`Major`/`Critical` ALWAYS takes EXACTLY ONE [`InjuryRng`] draw (even on an
/// empty/missing table — then discards it, for content-independent stream alignment).
/// The corpse-skip short-circuits BEFORE any draw, so a corpse takes neither the
/// severity nor the injury draw. The rolled `Option<RolledInjury>` freezes onto
/// [`HitReport::injury`].
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-438 threads the injury-roll inputs (the InjuryTables + InjuryRegistry \
              reads + the &mut InjuryRng draw stream) onto the wound fold alongside the \
              irreducible outcome / weapon / luck / target / entity / tuning / severity-rng \
              set; the target ganger surfaces are ALREADY grouped in the TargetGanger \
              bundle"
)]
fn fold_ganger(
    outcome: &ShotOutcome,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
    target: TargetGanger<'_>,
    target_entity: Entity,
    tuning: &CombatTuning,
    rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    // (1) Corpse-skip BEFORE any draw: a dead target is final — no draw is taken
    // (a corpse never consumes an RNG draw), nothing mutates.
    if *target.life == LifeState::Dead {
        return HitReport::no_effect(outcome.kind);
    }

    // (2) The struck part rode along on the §4 part roll (drawn upstream); a Ganger
    // outcome carries Some. A defensive None folds to a no-effect report (no draw).
    let Some(part) = outcome.body_part else {
        return HitReport::no_effect(outcome.kind);
    };

    // (3) Armored piece + matchup, or zeroed bare flesh under Neutral. The struck
    // piece is resolved by the caller from `ganger → Wears → the BodyPart-tagged piece`
    // (GTW-323 / ADR-0004); `struck_piece` reads its stats (the wear mutation comes
    // later in `apply_hit`, via the same piece's `&mut ArmorIntegrity`).
    let (piece, resolved_matchup) = struck_piece(target.piece.as_ref(), weapon);

    // (4) The per-hit damage formula (E3.3) — pure, mutates nothing.
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        resolved_matchup,
        tuning,
    );

    // (5) The ONE RNG draw: the wound-severity roll (E3.4). Both gangers' Luck, the
    // defender's Toughness, the struck part's mod, and the weapon's fatal bias feed
    // it; the injected SeverityRng is the draw point.
    let inputs = SeverityInputs::new(
        hit.penetrating,
        target.toughness,
        part_severity_mod(part),
        *weapon.fatal_bias,
        shooter_luck,
        target.luck,
    );
    let severity = roll_severity(&inputs, &tuning.severity_scaling, rng);

    // (6) Apply the resolved hit onto the target in place (E3.6) — HP loss +
    // Wounds-by-tier + armor wear + the terminal gates; capture the per-hit
    // ArmorWearOutcome (the break crossing / a non-breaking reduction / nothing).
    let wear_outcome = apply_hit(
        GangerHitTarget {
            hp:        target.hp,
            wounds:    target.wounds,
            life:      target.life,
            // The struck piece's `&mut ArmorIntegrity` (None on bare flesh / no piece)
            // — `apply_hit`'s `wear_armor` degrades it in place (GTW-323 / ADR-0004).
            integrity: target.piece.map(|p| p.integrity),
            inflicted: target.inflicted,
        },
        &hit,
        severity,
        part,
        target_entity,
        tuning,
    );

    // Map the mutually-exclusive outcome onto the report's two sibling armor fields:
    // a Broke hit sets `broken` (worn None), a Worn hit sets `worn` (broken None),
    // an Unaffected hit leaves BOTH None (GTW-313). At most one is ever Some.
    let (broken, worn) = match wear_outcome {
        ArmorWearOutcome::Broke(broken) => (Some(broken), None),
        ArmorWearOutcome::Worn(worn) => (None, Some(worn)),
        ArmorWearOutcome::Unaffected => (None, None),
    };

    // (6b) GTW-438 — the injury roll, AFTER apply_hit (Wounds already spent) and gated
    // on the rolled `severity` (the ticket's "after wounds-spend, before terminal
    // gates"). `roll_injury` takes EXACTLY ONE InjuryRng draw for a Minor/Major/Critical
    // wound (even on an empty/missing table — content-independent stream alignment) and
    // ZERO for a None (graze) / Fatal. It is recorded even on a corpse-MAKING hit (the
    // gate is on the rolled severity, not the post-gate life state); a hit on an already
    // dead target short-circuited at step (1) BEFORE any draw.
    let injury = roll_injury(part, severity, tables, registry, injury_rng);

    // (7) Freeze the verdict — named newtypes + the rolled injury, no pixel.
    HitReport {
        kind: outcome.kind,
        part: Some(part),
        applied: Some(AppliedDamage {
            matchup: resolved_matchup,
            hit,
            severity,
            life_after: *target.life,
            broken,
            worn,
        }),
        cover_destroyed: None,
        // A ganger hit destroys no slab (a slab hit takes the slab arm in
        // `resolve_and_apply`, never this ganger fold).
        slab_destroyed: None,
        // A ganger hit accrues no ground damage (a ground hit takes the ground arm in
        // `resolve_and_apply`, never this ganger fold).
        ground_accrued: None,
        // The GTW-438 injury verdict (Some only on a Minor/Major/Critical wound that
        // rolled a named injury; None on a graze / Fatal / empty-table).
        injury,
    }
}

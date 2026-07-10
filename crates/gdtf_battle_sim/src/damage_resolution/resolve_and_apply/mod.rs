//! The E3.9 **capstone integrator** — [`resolve_and_apply`] folds one
//! [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) → damage → severity →
//! application into ONE model-side act and returns a FROZEN per-hit report.
//!
//! This is the last E3 slice (`docs/combat/resolution.md` §5 / §6 / §"What's pure
//! math vs sim"; `docs/combat/weapons-and-armor.md` §"Per-hit resolution"). It
//! applies the result as ONE model-side act (armor → severity → application, with
//! corpse-skip draw discipline) and returns the frozen per-round reports — the
//! authoritative-model role this crate plays in the model/view split (ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! GTW-573 shape — one dispatch, one module per struck kind:
//!
//! - `fold` holds [`resolve_and_apply`], the sim's ONE logic-free delegation
//!   dispatch over [`ShotKind`](crate::resolve_coarse::ShotKind);
//! - `kinds` holds one module per struck kind (ganger / cover / slab / ground),
//!   each owning its whole fold AND its per-kind verdict payload type;
//! - `report` holds the frozen value types: the closed [`HitVerdict`] enum (one
//!   variant per kind — the old parallel per-kind `Option` bag is gone) and the
//!   [`HitReport`] (`kind` = what the round struck, `verdict` = what the fold did);
//! - `wound_core` holds the attacker-agnostic §5 → §6 → §8 wound-synthesis core
//!   the ganger kind and the no-attacker fall path share (GTW-523).
//!
//! The composed E3 verbs (matchup → `resolve_hit` → `roll_severity` → `apply_hit` →
//! `roll_injury`) are REUSED by the kind modules, never rebuilt. The severity-gated
//! draw discipline (one [`SeverityRng`](crate::rng::SeverityRng) draw per live ganger
//! hit; one severity-gated [`InjuryRng`](crate::rng::InjuryRng) draw on a tabled
//! wound only; zero draws everywhere else) is documented at the dispatch and pinned
//! by `test::draw_discipline`. [`resolve_and_apply`] is MODEL-side: it does **not**
//! charge TU or loop the burst (the E4 `fire()` act does), and it carries **no
//! pixel** — the report holds only damage / wound math, never a screen coordinate.

mod fold;
mod kinds;
mod report;
mod wound_core;

#[cfg(test)]
mod test;

pub use fold::resolve_and_apply;
/// The two shared cover-hit primitives the §7 melee cover-smash path reuses (GTW-508 C1):
/// the cover armor-piece shape and the HP-loss → cover-HP conversion. `pub(crate)` re-export
/// (the `kinds::cover` module owns them) so [`resolve_structural_melee`](crate::melee::resolve_structural_melee)
/// imports the ONE definition instead of copying the ranged cover-fold glue.
pub(crate) use kinds::cover::{cover_armor_piece, cover_damage_from_hp};
/// The per-kind verdict payload types, each owned by its struck-kind module (GTW-573
/// P10): the ganger wound verdict + applied-damage block, the cover / slab
/// destruction verdicts, and the ground accrual.
pub use kinds::{
    cover::CoverVerdict, ganger::AppliedDamage, ganger::GangerVerdict, ground::GroundAccrual,
    slab::SlabVerdict,
};
pub use report::{HitReport, HitVerdict, Protecting, StruckPiece, StruckSurfaces, TargetGanger};
/// The **attacker-agnostic wound-synthesis core** (GTW-523 remediation): the ONE shared
/// §5 → §6 → §8 fold both the ganger kind module and the no-attacker fall path
/// ([`resolve_fall_hit`](crate::falls::resolve_fall_hit)) route through, plus its
/// input bundle + blow value types. `pub(crate)` re-export (the private `wound_core` module
/// owns them) so the falls fork imports the ONE definition instead of re-running the
/// `resolve_hit` → `roll_severity` → `apply_hit` → `roll_injury` orchestration — the two
/// paths therefore cannot drift. The core's [`WoundSynthesis`](wound_core::WoundSynthesis)
/// verdict stays module-local (each caller consumes it in place), so it is not re-exported.
pub(crate) use wound_core::{WoundBlow, WoundCoreInputs, synthesize_wound};

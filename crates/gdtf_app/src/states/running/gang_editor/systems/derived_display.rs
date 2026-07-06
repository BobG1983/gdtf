//! The shared derived-stat FORMATTER (GTW-428 C3) — the single place a
//! [`DerivedStats`](gdtf_battle_sim::DerivedStats) field becomes the string a readonly
//! [`DerivedStatText`](super::super::components::DerivedStatText) node shows.
//!
//! Both the initial seed (in [`spawn`](super::spawn)) and the live recompute (in
//! [`attribute_edit`](super::attribute_edit)) format through THIS function, so the spawned text and
//! the recomputed text are bit-identical for equal stats — the C3 "displayed == pipeline(attrs)"
//! contract has exactly ONE rendering, never two that could drift.

use gdtf_battle_sim::DerivedStats;

use crate::states::running::gang_editor::components::DerivedStat;

/// How many fractional digits the f32 SKILL stats (Shooting / Fight / Reactions / Morale) render
/// with — the integer POOLS (TU / HP / Wounds / Bottle) render with none. Framework-plumbing
/// formatting precision (fed to a `format!` width), not a domain value.
const SKILL_DECIMALS: usize = 1;

/// Format one [`DerivedStat`] of `stats` as the string its readonly display shows (GTW-428 C3).
///
/// The four f32 SKILL stats render to [`SKILL_DECIMALS`] decimals; the four integer POOLS render
/// as their bare integer. Every derived newtype `Deref`s to its inner scalar, so the value is read
/// through the deref and formatted — no reimplementation of the GTW-384 derivation, only its
/// presentation.
#[must_use]
pub(in crate::states::running::gang_editor) fn format_derived(
    stats: &DerivedStats,
    stat: DerivedStat,
) -> String {
    match stat {
        DerivedStat::Shooting => format!("{:.SKILL_DECIMALS$}", *stats.shooting),
        DerivedStat::Fight => format!("{:.SKILL_DECIMALS$}", *stats.fight),
        DerivedStat::Reactions => format!("{:.SKILL_DECIMALS$}", *stats.reactions),
        DerivedStat::Morale => format!("{:.SKILL_DECIMALS$}", *stats.morale),
        DerivedStat::Tu => format!("{}", *stats.tu),
        DerivedStat::Hp => format!("{}", *stats.hp),
        DerivedStat::Wounds => format!("{}", *stats.wounds),
        DerivedStat::Bottle => format!("{}", *stats.bottle),
    }
}

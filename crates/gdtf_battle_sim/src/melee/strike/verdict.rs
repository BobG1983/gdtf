//! The frozen verdict one [`resolve_melee_strike`](super::resolve_melee_strike) produces —
//! what the owning dispatcher reads to decide its output signals.

use crate::{
    armor_wear::ArmorWearOutcome, injuries::RolledInjury, melee::Connected, resolve_hit::HpDamage,
    severity::Severity,
};

/// The frozen verdict one [`resolve_melee_strike`](super::resolve_melee_strike) produces —
/// whether the strike connected and, on a connect, the rolled [`Severity`], the applied HP
/// loss, the armor-wear outcome, and the rolled injury (GTW-507 / GTW-572 / GTW-821).
///
/// A value object of named domain types (no bare primitive, no pixel) the owning
/// [`dispatch_melee`](crate::acts::dispatch_melee) reads to decide its output: a `connect`
/// (the §7 `atk > def` verdict) is the gate the presenter [`MeleeResolved`](crate::acts::MeleeResolved)
/// signal + the per-strike effects key off, `severity` is the rolled §6 bucket on a
/// connecting hit (the wound the §6 step already spent onto the target), `hp_damage`
/// is the SCALED per-hit HP loss the §6 fold applied (GTW-572 — surfaced so the
/// [`MeleeStruck`](crate::acts::MeleeStruck) fact can carry the damage number), and
/// `injury` is the §8 draw the shared wound core rolled (GTW-821 — bridged by the dispatch
/// into the EXISTING [`InjuryInflicted`](crate::acts::InjuryInflicted) message, exactly as
/// the fire and fall paths bridge theirs). A miss carries `connect == false`,
/// [`Severity::None`], a zero `hp_damage`, and no injury (nothing was applied).
///
/// `Clone`, not `Copy` — the [`RolledInjury`] owns a `Vec` of effects + three texts (the
/// [`GangerVerdict`](crate::resolve_and_apply::GangerVerdict) precedent).
#[derive(Debug, Clone, PartialEq)]
pub struct MeleeStrike {
    /// Whether the §7 opposed roll connected (`atk > def`) — the strike landed.
    pub connect:   Connected,
    /// The rolled §6 wound severity on a connecting hit; [`Severity::None`] on a miss.
    pub severity:  Severity,
    /// The §5→§7-multiplied HP loss the connecting hit applied; zero on a miss (GTW-572).
    pub hp_damage: HpDamage,
    /// The §6 armor-wear outcome the fold applied onto the struck worn piece —
    /// [`ArmorWearOutcome::Unaffected`] on a miss (GTW-572: previously computed and dropped
    /// inside the verb, now surfaced so a melee armor BREAK emits the
    /// [`ArmorBroken`](crate::armor_wear::ArmorBroken) fact like a ranged one).
    pub wear:      ArmorWearOutcome,
    /// The named injury the §8 roll drew for this wound (GTW-821) — `Some` ONLY on a
    /// non-graze, non-fatal connecting wound whose melee weighting bucket rolled one;
    /// `None` on a miss / graze / fatal, or an empty-or-missing bucket (which still took
    /// its one [`InjuryRng`](crate::rng::InjuryRng) draw — the content-independent
    /// stream-alignment property).
    pub injury:    Option<RolledInjury>,
}

impl MeleeStrike {
    /// A **missed** strike — the opposed roll was lost, so no damage was applied, no
    /// severity was drawn ([`Severity::None`], zero HP loss), no armor was worn, and no
    /// injury was rolled.
    pub(super) const MISS: Self = Self {
        connect:   Connected::new(false),
        severity:  Severity::None,
        hp_damage: HpDamage::new(0),
        wear:      ArmorWearOutcome::Unaffected,
        injury:    None,
    };

    /// A **connected but inert** strike — the §7 roll landed, yet the shared wound core
    /// applied nothing because the target was already a corpse (its corpse-skip returns
    /// before any §6 / §8 draw and mutates nothing).
    ///
    /// Unreachable through the live act (`resolve_ganger_melee` gates on an ACTIVE target
    /// before it strikes); it exists so the verb reports the corpse-skip honestly — the
    /// connect verdict the §7 roll produced, with zero applied damage, no wound, and no
    /// injury — instead of claiming damage that was never folded on.
    pub(super) const CORPSE: Self = Self {
        connect:   Connected::new(true),
        severity:  Severity::None,
        hp_damage: HpDamage::new(0),
        wear:      ArmorWearOutcome::Unaffected,
        injury:    None,
    };
}

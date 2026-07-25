//! The connect-side output of one ganger-vs-ganger melee strike — the presenter signal, the
//! GTW-572 facts, the GTW-821 injury bridge, the GTW-547 death gate, and the GTW-525 shove.

use bevy::prelude::{Entity, MessageWriter};

use super::{MeleeFacts, queries::MeleeTargetQuery, snapshot::AttackerSnapshot};
use crate::{
    acts::{
        InjuryInflicted,
        request::{MeleeResolved, MeleeStruck, ShoveRequested},
    },
    armor_wear::ArmorWearOutcome,
    effects::on_death::OnDeathOccurred,
    ganger::LifeState,
    melee::MeleeStrike,
    metric::CellLevel,
};

/// The buffered writers one CONNECTING ganger strike emits through, bundled by `&mut` so
/// [`emit_connect_signals`] stays under clippy's argument-count gate with NO suppression (the
/// falls `FallSignals` precedent).
///
/// A transparent borrow bundle of the named writers (framework plumbing — no bare domain
/// value). Lifetime `'a` ties the borrows to the resolver's frame; each writer keeps its OWN
/// world lifetime (`'r` / `'f` / `'s` / `'d`) because a `&mut` is invariant over its type —
/// forcing one shared world lifetime across four independently-borrowed system params does
/// not type-check.
pub(super) struct MeleeConnectSignals<'a, 'r, 'f, 's, 'd> {
    /// The presenter strike-glyph signal — one per connecting strike (GTW-507).
    pub(super) resolved: &'a mut MessageWriter<'r, MeleeResolved>,
    /// The GTW-572 number-bearing fact + armor-break fact + the GTW-821 injury message.
    pub(super) facts:    &'a mut MeleeFacts<'f>,
    /// The GTW-525 weapon-tag auto-shove request.
    pub(super) shoves:   &'a mut MessageWriter<'s, ShoveRequested>,
    /// The GTW-547 terminal-death signal for a strike that killed the target.
    pub(super) deaths:   &'a mut MessageWriter<'d, OnDeathOccurred>,
}

/// Emit everything a CONNECTING ganger strike produces, off the frozen [`MeleeStrike`] verdict
/// and the target's post-fold state — factored out of
/// [`resolve_ganger_melee`](super::ganger::resolve_ganger_melee) so each stays one concern:
///
/// - the presenter [`MeleeResolved`] strike-glyph at the target's cell (GTW-507);
/// - the number-bearing [`MeleeStruck`] fact the combat log's melee-damage line reads
///   (GTW-572);
/// - the [`ArmorBroken`](crate::armor_wear::ArmorBroken) fact on a protecting→broken wear
///   crossing, so a melee break pops and logs exactly like a ranged one (GTW-572);
/// - the [`InjuryInflicted`] bridge for the §8 injury the shared wound core rolled — the SAME
///   message the fire path (`dispatch_fire`) and the fall path (`apply_falls`) emit, which the
///   `apply_injury` boundary folds onto the target's injury ledger (GTW-821);
/// - the [`OnDeathOccurred`] terminal gate when the fold left the target Dead (GTW-547);
/// - the internal [`ShoveRequested`] a `shove`-tagged weapon triggers (GTW-525).
///
/// A MISS never reaches here (the caller gates on the §7 connect), so every write is
/// connect-only. Takes the verdict BY VALUE — the rolled injury is moved into the message.
pub(super) fn emit_connect_signals(
    attacker: &AttackerSnapshot<'_>,
    target_entity: Entity,
    at: CellLevel,
    strike: MeleeStrike,
    targets: &MeleeTargetQuery,
    signals: MeleeConnectSignals<'_, '_, '_, '_, '_>,
) {
    signals
        .resolved
        .write(MeleeResolved::new(at, attacker.strike_damage_type));

    // GTW-572: the NUMBER-BEARING melee fact — the connecting strike's applied HP loss, with
    // both combatants, so the combat log can phrase a melee-damage line. The strike-glyph
    // signal above stays cell+damage-type only (the GTW-507 FX contract).
    signals.facts.struck.write(MeleeStruck::new(
        attacker.entity,
        target_entity,
        strike.hp_damage,
    ));

    // GTW-572: a protecting→broken wear crossing (surfaced on the strike verdict) emits the
    // SAME ArmorBroken fact a ranged break does.
    if let ArmorWearOutcome::Broke(broken) = strike.wear {
        signals.facts.breaks.write(broken);
    }

    // GTW-821: the §8 injury the shared wound core rolled for this wound bridges into the
    // EXISTING InjuryInflicted message, addressed to the struck ganger — the `apply_injury`
    // boundary then folds it onto the target's ledger (the dispatch_fire / apply_falls bridge
    // precedent). A graze / fatal / empty-bucket wound rolled none and emits nothing.
    if let Some(rolled) = strike.injury {
        signals
            .facts
            .injuries
            .write(InjuryInflicted::from_rolled(target_entity, rolled));
    }

    // GTW-547: a CONNECTING strike that KILLED the target (its post-fold LifeState is Dead)
    // emits the terminal-death signal at the target's cell so `resolve_on_death` fans the dead
    // ganger's on-death effect. Re-read the target's LifeState off the query AFTER the in-place
    // fold (the SINGLE LifeState read path); a strike that wounded-but-did-not-kill emits
    // nothing.
    if targets
        .get(target_entity)
        .is_ok_and(|(_, _, &life, ..)| life == LifeState::Dead)
    {
        signals
            .deaths
            .write(OnDeathOccurred::new(target_entity, at));
    }

    // GTW-525 C3: a `shove`-tagged weapon KNOCKS BACK the target on a CONNECTING strike (in
    // addition to the damage above). The connect already gated + charged, so dispatch_shove
    // resolves it un-gated / TU-free (ShoveSource::Weapon); it is ordered `.after(dispatch_melee)`,
    // so the same-frame message is consumed this tick.
    if *attacker.shove {
        signals
            .shoves
            .write(ShoveRequested::new_weapon(attacker.entity, target_entity));
    }
}

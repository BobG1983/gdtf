//! The [`InjuryInflicted`] message + the [`apply_injury`] boundary system — the GTW-438
//! seam that turns a frozen [`HitReport::injury`](crate::resolve_and_apply::HitReport::injury)
//! roll into a persistent injury on the wounded ganger.
//!
//! The roll itself is PURE + in-fold ([`roll_injury`](crate::injuries::roll_injury), frozen
//! onto the report); the side effects live HERE, at the message boundary (mirroring the
//! `CoverDestroyed` / `SlabDestroyed` bridge precedent, `bevy-traps.md` #7 — no `&mut
//! World`):
//!
//! 1. [`dispatch_fire`](crate::acts::dispatch_fire) emits one [`InjuryInflicted`] per
//!    fired round whose [`HitReport::injury`](crate::resolve_and_apply::HitReport::injury)
//!    is `Some`, carrying the wounded target [`Entity`] + the rolled [`GainedInjury`] (the
//!    durable ledger entry) + the three transient texts the presenter (GTW-439) routes.
//! 2. [`apply_injury`] drains that buffer and, for each message, appends the
//!    [`GainedInjury`] to the target's [`InflictedInjuries`] via
//!    [`gain`](InflictedInjuries::gain) (tripping `Changed`, which the GTW-436
//!    `rederive_stats_on_injury_change` projector reacts to), then keeps the standalone
//!    [`BleedAfflicted`](crate::injuries::BleedAfflicted) component in sync with the ledger's accrued bleed (so the bleed
//!    runtime can query it).

use bevy::prelude::{Commands, Entity, Message, MessageReader, Query};

use crate::{
    armor::BodyPart,
    injuries::{
        GainedInjury, InflictedInjuries, InjuryName, InspectText, LogText, PopupText, RolledInjury,
    },
    severity::Severity,
};

/// One **injury was inflicted** — the GTW-438 boundary message bridging a frozen
/// [`HitReport::injury`](crate::resolve_and_apply::HitReport::injury) roll into the
/// [`apply_injury`] applier + the presenter's transient flash (GTW-439).
///
/// Emitted once per fired round whose report rolled a named injury, by
/// [`dispatch_fire`](crate::acts::dispatch_fire) (the cover-/slab-destroyed bridge
/// precedent). It carries everything the applier + presenter need: the wounded
/// [`target`](InjuryInflicted::target) [`Entity`], the durable [`gained`](InjuryInflicted::gained)
/// ledger entry the applier folds in, and the three transient texts
/// ([`popup_text`](InjuryInflicted::popup_text) → FCT, [`log_text`](InjuryInflicted::log_text)
/// → combat log, [`inspect_text`](InjuryInflicted::inspect_text) → the flash) the
/// presenter (GTW-439) routes — the ledger drives the PERSISTENT inspect list, this
/// message drives the ONE-SHOT flash.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`ShotFired`](crate::shot_fired::ShotFired) / [`Bleeding`](crate::bleed::Bleeding) /
/// [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed). The
/// [`target`](InjuryInflicted::target) is a Bevy [`Entity`] handle (framework plumbing,
/// the only bare type the no-bare-types rule permits in a payload); every other field is
/// a named domain value. NOT `Copy` — the [`gained`](InjuryInflicted::gained) entry holds
/// an owned `Vec` of effects + texts.
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-444): its [`GainedInjury`] effects may carry a
/// [`MovementCostMul`](crate::injuries::InjuryEffect::MovementCostMul) whose `f32` payload
/// is not `Eq`. A buffered message is read in order, never compared in a hashed/ordered set.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct InjuryInflicted {
    /// The wounded ganger — the entity whose [`InflictedInjuries`] gains the injury.
    pub target:       Entity,
    /// The durable ledger entry the applier folds into the target's
    /// [`InflictedInjuries`] (the name / struck part / severity / frozen effects + the
    /// inspect text — the GTW-436 projector's delta source + the future GTW-23 heal input).
    pub gained:       GainedInjury,
    /// The injury's display name (also on [`gained`](InjuryInflicted::gained); surfaced
    /// here so the presenter need not re-read the ledger for the flash).
    pub name:         InjuryName,
    /// The struck body part.
    pub part:         BodyPart,
    /// The rolled severity bucket (`Minor` / `Major` / `Critical`).
    pub severity:     Severity,
    /// The FCT popup line (transient — drives the one-shot flash, NOT carried on the
    /// durable ledger).
    pub popup_text:   PopupText,
    /// The combat-log clause (transient).
    pub log_text:     LogText,
    /// The inspect-panel description (also durable on the ledger; carried here for the flash).
    pub inspect_text: InspectText,
}

impl InjuryInflicted {
    /// Build an [`InjuryInflicted`] for `target` from a frozen [`RolledInjury`] — the
    /// fire-act bridge (GTW-438).
    ///
    /// Splits the roll into the durable [`GainedInjury`] ledger entry (via
    /// [`RolledInjury::into_gained`], which drops the transient texts) and the three
    /// routed texts (cloned off the roll before it is consumed), so the ONE message
    /// carries both the applier's input and the presenter's flash payload.
    #[must_use]
    pub fn from_rolled(target: Entity, rolled: RolledInjury) -> Self {
        // Clone the fields the message surfaces BEFORE `into_gained` consumes the roll
        // (the gained entry keeps the durable name/part/severity/effects/inspect text).
        let name = rolled.name.clone();
        let part = rolled.part;
        let severity = rolled.severity;
        let popup_text = rolled.popup_text.clone();
        let log_text = rolled.log_text.clone();
        let inspect_text = rolled.inspect_text.clone();
        Self {
            target,
            gained: rolled.into_gained(),
            name,
            part,
            severity,
            popup_text,
            log_text,
            inspect_text,
        }
    }
}

/// **Apply** every buffered [`InjuryInflicted`] to its target ganger — the GTW-438
/// boundary system (`docs/combat/resolution.md` injury tables; the cover-/slab-destroyed
/// bridge precedent, `bevy-traps.md` #7 — query / `Commands` / `MessageReader`, no `&mut
/// World`).
///
/// For each drained message:
///
/// 1. **Gain** — append the [`GainedInjury`] to the target's [`InflictedInjuries`] via
///    [`gain`](InflictedInjuries::gain), which folds each effect through the SINGLE
///    exhaustive match (a [`Modify`](crate::injuries::InjuryEffect::Modify) sums into the
///    per-stat delta the GTW-436 projector reads; a
///    [`Bleeding`](crate::injuries::InjuryEffect::Bleeding) accrues into the ledger's
///    bleed total). The append trips `Changed<InflictedInjuries>`, which the GTW-436
///    `rederive_stats_on_injury_change` projector reacts to — so the stat deltas land on
///    the derived stats the same tick (it is ordered `.after(SimSystems::Simulate)`).
/// 2. **Ensure the ledger exists** — a ganger MUST carry [`InflictedInjuries`] to
///    [`gain`](InflictedInjuries::gain) into. The spawn seeds it (the `bsn!` sentinel
///    `Default`, GTW-438); this system ALSO inserts-if-absent via [`Commands`] as a
///    no-panic backstop for any ganger spawned without it (e.g. a bare-`World` test
///    spawn) — building the ledger from the message's `gained` entry rather than dropping
///    the injury.
/// 3. **Sync the bleed component** — keep the standalone [`BleedAfflicted`](crate::injuries::BleedAfflicted) component
///    equal to the ledger's accrued bleed (the SINGLE source is the ledger's `gain`; this
///    mirror is what the bleed runtime queries), inserted via [`Commands`].
///
/// Param-only (`bevy-traps.md` #7): a [`MessageReader`] (drain), a [`Query`] for the
/// target's ledger, and [`Commands`] (the insert-if-absent + the bleed-sync) — no `&mut
/// World`. A message whose target is despawned or absent is a no-op (fail-closed). The
/// applier takes NO RNG draw — the roll already happened in-fold (the determinism
/// property).
pub fn apply_injury(
    mut inflicted: MessageReader<InjuryInflicted>,
    mut ledgers: Query<&mut InflictedInjuries>,
    mut commands: Commands,
) {
    for message in inflicted.read() {
        let target = message.target;
        // A despawned target — skip (fail-closed; the entity may have been removed
        // between fire resolution and this applier).
        let Ok(mut entity) = commands.get_entity(target) else {
            continue;
        };

        // (1) + (2) Gain into the EXISTING ledger if present; else build a fresh ledger
        //     from this entry and insert it (the no-panic backstop for a ganger spawned
        //     without the sentinel-seeded component). Either path folds the effects via
        //     the ledger's single exhaustive `gain` match.
        let bleed = if let Ok(mut ledger) = ledgers.get_mut(target) {
            ledger.gain(message.gained.clone());
            ledger.bleed()
        } else {
            let mut ledger = InflictedInjuries::default();
            ledger.gain(message.gained.clone());
            let bleed = ledger.bleed();
            // Insert the freshly-built ledger so the projector + inspect can read it.
            entity.insert(ledger);
            bleed
        };

        // (3) Keep the standalone BleedAfflicted component in sync with the ledger's
        //     accrued bleed (the bleed runtime queries this component). Inserting the
        //     SAME value the ledger now holds keeps the single-source invariant: the
        //     ledger's `gain` is the only accrual, this is a queryable mirror.
        entity.insert(bleed);
    }
}

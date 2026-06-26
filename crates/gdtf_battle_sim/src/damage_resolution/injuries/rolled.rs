//! The two injury verdict records — the in-fold [`RolledInjury`] (frozen on the
//! `HitReport`) and the persistent [`GainedInjury`] (the ledger entry).

use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::BodyPart, severity::Severity};

/// The frozen verdict of one injury roll — what the in-fold roll
/// (`roll_injury`, GTW-438) produces and freezes onto `HitReport.injury`
/// (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// A snapshot of the rolled [`InjuryDef`](super::InjuryDef)'s applied state at the
/// moment of infliction: the [`name`](RolledInjury::name), the struck
/// [`part`](RolledInjury::part), the rolled [`severity`](RolledInjury::severity),
/// the FROZEN [`effects`](RolledInjury::effects) (frozen so a later content
/// hot-edit can't rewrite an already-suffered injury), and the three routed texts.
/// The fire-act bridge (GTW-437) turns this into an `InjuryInflicted` message and a
/// [`GainedInjury`] ledger entry. Public fields (a value-object record). THIS slice
/// only names the type — no roll site exists yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolledInjury {
    /// The display name of the rolled condition.
    pub name:         InjuryName,
    /// The struck body part.
    pub part:         BodyPart,
    /// The rolled severity bucket (`Minor` / `Major` / `Critical`).
    pub severity:     Severity,
    /// The frozen effects to apply (snapshotted at roll time).
    pub effects:      Vec<InjuryEffect>,
    /// The FCT popup line.
    pub popup_text:   PopupText,
    /// The combat-log clause.
    pub log_text:     LogText,
    /// The persistent inspect-panel description.
    pub inspect_text: InspectText,
}

impl RolledInjury {
    /// Build a rolled-injury verdict from its parts (the roll site, GTW-438).
    #[must_use]
    pub const fn new(
        name: InjuryName,
        part: BodyPart,
        severity: Severity,
        effects: Vec<InjuryEffect>,
        popup_text: PopupText,
        log_text: LogText,
        inspect_text: InspectText,
    ) -> Self {
        Self {
            name,
            part,
            severity,
            effects,
            popup_text,
            log_text,
            inspect_text,
        }
    }

    /// Convert this transient roll verdict into a persistent [`GainedInjury`] ledger
    /// entry, dropping the transient FCT/log texts (those drive the one-shot
    /// `InjuryInflicted` flash; the ledger keeps only the durable inspect text and
    /// the frozen effects).
    #[must_use]
    pub fn into_gained(self) -> GainedInjury {
        GainedInjury::new(
            self.name,
            self.part,
            self.severity,
            self.effects,
            self.inspect_text,
        )
    }
}

/// One **gained injury** carried on a ganger's [`InflictedInjuries`](super::InflictedInjuries)
/// ledger — the persistent, campaign-carried record of an injury it has suffered
/// (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// The durable counterpart of a [`RolledInjury`]: it keeps the FROZEN
/// [`effects`](GainedInjury::effects) (so the GTW-436 projector re-sums them every
/// projection and a future GTW-23 heal removes them by removing this entry) and the
/// [`inspect_text`](GainedInjury::inspect_text) (the inspect-panel list source),
/// dropping the transient FCT/log texts. Public fields (a value-object record). The
/// sole mutator of the ledger is [`InflictedInjuries::gain`](super::InflictedInjuries::gain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainedInjury {
    /// The display name of the suffered condition.
    pub name:         InjuryName,
    /// The struck body part.
    pub part:         BodyPart,
    /// The rolled severity bucket.
    pub severity:     Severity,
    /// The frozen effects (re-summed by the projector; removed on a future heal).
    pub effects:      Vec<InjuryEffect>,
    /// The persistent inspect-panel description.
    pub inspect_text: InspectText,
}

impl GainedInjury {
    /// Build a gained-injury ledger entry from its parts.
    #[must_use]
    pub const fn new(
        name: InjuryName,
        part: BodyPart,
        severity: Severity,
        effects: Vec<InjuryEffect>,
        inspect_text: InspectText,
    ) -> Self {
        Self {
            name,
            part,
            severity,
            effects,
            inspect_text,
        }
    }
}

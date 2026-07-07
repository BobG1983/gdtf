//! The INJURY-mode def form's **working model** (GTW-654): the state-scoped
//! [`InjuryDraft`] resource the egui form's controls write and the save reads.
//!
//! The draft holds the injury's KEY (a text-field buffer — the file stem minus the
//! `.injury` infix IS the registry key, the GTW-437 stem-key model) plus the sim's
//! own [`InjuryDef`] record, so the edited model IS the loader schema — projecting
//! to the save is a copy, never a parallel schema (the GTW-636 gang-draft / GTW-479
//! armor-draft precedent: holding the sim type means every authored field — incl.
//! the closed-palette effects list — round-trips by construction).

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        InjuryDef, InjuryEffect, InjuryName, InspectText, LogText, PopupText, PostHeal, StatDelta,
        StatTarget,
    },
    severity::Severity,
};

/// The default effect ROW the form seeds — both the "Add effect" press and a fresh
/// draft's one seed row (the `.injury.ron` schema documents `effects` as `≥ 1`, so a
/// brand-new def is schema-conformant by construction). A mild single-stat debuff
/// the author immediately re-targets/re-tunes; carries no shipped-content meaning.
pub(crate) const DEFAULT_EFFECT: InjuryEffect = InjuryEffect::Modify {
    stat:   StatTarget::Speed,
    amount: StatDelta::new(-1),
};

/// Whether a form's ONE-SHOT open-with-content seed has run yet (GTW-654) — the
/// GTW-479 Armor-mode lifecycle, shared by the injury def draft and the weighting
/// draft. A closed enum (no-bare-types — a lifecycle phase is a domain value, not a
/// bare `bool`): [`Pending`](AutoloadState::Pending) until the shell has seen the
/// resolved registry/tables, then [`Done`](AutoloadState::Done) forever (a "New
/// injury" press / an explicit load must never be clobbered by a late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AutoloadState {
    /// The one-shot registry/tables seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The fresh seed [`InjuryDef`] a pristine / "New injury" draft holds: a Minor Head
/// injury with empty texts and the one [`DEFAULT_EFFECT`] row — every field the
/// author immediately edits; nothing here pins shipped content.
fn seed_def() -> InjuryDef {
    InjuryDef {
        name:         InjuryName::new(String::new()),
        category:     InjuryCategory::Head,
        severity:     Severity::Minor,
        popup_text:   PopupText::new(String::new()),
        log_text:     LogText::new(String::new()),
        inspect_text: InspectText::new(String::new()),
        effects:      vec![DEFAULT_EFFECT],
        post_heal:    PostHeal::Deferred,
    }
}

/// The in-progress INJURY-mode def DRAFT — the state-scoped resource the egui form's
/// controls write and the save projects into the loader's `(`[`InjuryName`]` key,
/// `[`InjuryDef`]`)` pair (GTW-654 C1).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed
/// `OnExit(Editing)` — bevy-traps #1). The key is a bare [`String`] only as the
/// text-field buffer (the [`ArmorDraft`](crate::armor_form::ArmorDraft) `name`
/// exception); it folds into the stem-role [`InjuryName`] on projection. The def is
/// the sim's own [`InjuryDef`] record (see the module docs). Private fields with
/// named accessors / mutators (no-bare-types rule 5): the def lifecycle (load / new)
/// is owned here, while per-field edits go through the sim record's own public
/// fields via [`def_mut`](InjuryDraft::def_mut) (the gang `members_mut` split).
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct InjuryDraft {
    /// The injury's KEY buffer — the registry key / file stem the save sanitizes.
    key:      String,
    /// The injury's editable def — the loader-schema record itself.
    def:      InjuryDef,
    /// The one-shot open-with-an-injury seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl InjuryDraft {
    /// A fresh draft for a NEW injury: an empty key, the `seed_def` shape (one
    /// default effect row) — the "New injury" press. Autoload is `Done`: a deliberate
    /// new def must never be clobbered by the one-shot registry seed (the
    /// [`ArmorDraft::new_armor`](crate::armor_form::ArmorDraft) parity).
    #[must_use]
    pub fn new_injury() -> Self {
        Self {
            key:      String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-an-injury seed is still pending — the shell
    /// checks this each Injury-mode frame and runs the autoload exactly once
    /// (idempotent under the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the Gang / Armor modes' "nothing loaded — start empty" parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing injury into the form (the load `ComboBox` / autoload path):
    /// the registry KEY becomes the key buffer and the def is copied in as the
    /// working record. Marks the one-shot seed done.
    pub fn load_injury(&mut self, key: &InjuryName, def: &InjuryDef) {
        key.as_str().clone_into(&mut self.key);
        self.def = def.clone();
        self.autoload = AutoloadState::Done;
    }

    /// The injury's current KEY buffer (the file stem / registry key).
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Set the injury's key (committed from the text field).
    pub fn set_key(&mut self, key: String) {
        self.key = key;
    }

    /// The injury's current def (read-only — the save projection).
    #[must_use]
    pub const fn def(&self) -> &InjuryDef {
        &self.def
    }

    /// The injury's def, mutably — per-field edits (display name, category,
    /// severity, texts, the effects list's add/edit/remove) go through the sim
    /// record's own public fields, while the def lifecycle stays owned by
    /// [`new_injury`](InjuryDraft::new_injury) / [`load_injury`](InjuryDraft::load_injury).
    #[must_use]
    pub const fn def_mut(&mut self) -> &mut InjuryDef {
        &mut self.def
    }
}

impl Default for InjuryDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot
    /// open-with-an-injury autoload still `Pending` — the shell seeds it from the
    /// resolved [`InjuryRegistry`](gdtf_battle_sim::injuries::InjuryRegistry) on the
    /// first Injury-mode frame (or marks it done when no injury is loaded).
    fn default() -> Self {
        Self {
            key:      String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Pending,
        }
    }
}

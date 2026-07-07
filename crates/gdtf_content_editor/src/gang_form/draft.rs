//! The GANG-mode form's **working model** (GTW-636): the state-scoped [`GangDraft`]
//! resource the egui form's controls write and the save reads.
//!
//! The draft holds the gang's NAME (a text-field buffer — the file stem IS the registry
//! key, the GTW-415 key model) plus its member list as the sim's own
//! [`GangMember`] records, so the edited model IS the loader schema — projecting to a
//! [`GangRoster`](gdtf_battle_sim::ganger::GangRoster) is a member-list copy, never a
//! parallel schema (the GTW-429 round-trip contract). Unlike the retired in-game
//! editor's `EditableMember` mirror, holding [`GangMember`] directly also carries the
//! GTW-505 `melee_weapon` key, so loading and re-saving a gang that authored one can
//! never silently drop it.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorName,
    ganger::{GangMember, GangName, GangRoster, GangerName},
    weapon::WeaponName,
};

/// The default new-member display name "Add member" stamps onto a fresh record — a named
/// placeholder so a freshly-added member reads as a member rather than blank (the retired
/// in-game editor's convention, kept for parity).
const DEFAULT_MEMBER_NAME: &str = "New Member";

/// Whether the Gang mode's ONE-SHOT open-with-a-gang seed has run yet (GTW-636).
///
/// The retired in-game editor opened with the FIRST gang (sorted by name) already loaded;
/// the Gang mode keeps that behavior via a one-shot autoload the shell runs on the first
/// Gang-mode frame. A closed enum (no-bare-types — a lifecycle phase is a domain value,
/// not a bare `bool`): [`Pending`](AutoloadState::Pending) until the shell has seen a
/// resolved [`GangRegistry`](gdtf_battle_sim::ganger::GangRegistry), then
/// [`Done`](AutoloadState::Done) forever (a "New gang" press must never be clobbered by a
/// late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    /// The one-shot registry seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The in-progress GANG-mode authoring DRAFT — the state-scoped resource the egui form's
/// controls write and the save projects into the loader's `(`[`GangName`]`,
/// `[`GangRoster`]`)` pair (GTW-636 C1).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). The name is a bare [`String`] only as the text-field buffer (the
/// [`ThemeDraft`](crate::theme_form::ThemeDraft) `display_name` exception); it folds into
/// a [`GangName`] on projection. The members are the sim's own [`GangMember`] records
/// (see the module docs). Private fields with named accessors / mutators
/// (no-bare-types rule 5): the LIST structure (add / remove / load) is owned here, while
/// per-field member edits go through the sim record's own public fields via
/// [`members_mut`](GangDraft::members_mut).
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GangDraft {
    /// The gang's NAME buffer — the registry key / file stem the save sanitizes.
    name:     String,
    /// The gang's editable member list — the loader-schema records themselves.
    members:  Vec<GangMember>,
    /// The one-shot open-with-a-gang seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl GangDraft {
    /// A fresh draft for a NEW gang: an empty name, no members — the "New gang" press.
    /// Autoload is `Done`: a deliberate new gang must never be clobbered by the
    /// one-shot registry seed.
    #[must_use]
    pub const fn new_gang() -> Self {
        Self {
            name:     String::new(),
            members:  Vec::new(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-a-gang seed is still pending — the shell checks
    /// this each Gang-mode frame and runs the autoload exactly once
    /// (idempotent under the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the retired editor's "no gangs loaded — start empty" behavior).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing gang into the form (the load `ComboBox` / autoload path): the
    /// registry KEY becomes the name buffer and the roster's members are copied in as
    /// the working list. Marks the one-shot seed done.
    pub fn load_gang(&mut self, name: &GangName, roster: &GangRoster) {
        name.as_str().clone_into(&mut self.name);
        self.members.clone_from(&roster.members);
        self.autoload = AutoloadState::Done;
    }

    /// The gang's current NAME buffer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the gang's name (committed from the text field).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// The gang's current members (read-only — the count label / save projection).
    #[must_use]
    pub fn members(&self) -> &[GangMember] {
        &self.members
    }

    /// The gang's current members, mutably, as a SLICE — per-field edits (name /
    /// attributes / weapon / armor / melee keys) go through the sim record's own public
    /// fields, while the LIST structure stays owned by
    /// [`add_member`](GangDraft::add_member) / [`remove_member`](GangDraft::remove_member).
    #[must_use]
    pub fn members_mut(&mut self) -> &mut [GangMember] {
        &mut self.members
    }

    /// Append a fresh default member (the "Add member" press): the placeholder name,
    /// every attribute at its zero default, empty weapon / armor keys, and no authored
    /// melee weapon (resolves to the `fists` default in-game) — the retired editor's
    /// default-member shape, kept for parity.
    pub fn add_member(&mut self) {
        self.members.push(GangMember {
            name:         GangerName::new(DEFAULT_MEMBER_NAME.to_owned()),
            speed:        default(),
            aim:          default(),
            strength:     default(),
            toughness:    default(),
            reflexes:     default(),
            cool:         default(),
            grit:         default(),
            luck:         default(),
            armor:        ArmorName::new(String::new()),
            weapon:       WeaponName::new(String::new()),
            melee_weapon: None,
        });
    }

    /// Remove the member at `index`, returning `true` when one was actually removed —
    /// a no-op returning `false` for an out-of-range index (degraded, never panics).
    pub fn remove_member(&mut self, index: usize) -> bool {
        if index < self.members.len() {
            self.members.remove(index);
            true
        } else {
            false
        }
    }
}

impl Default for GangDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot open-with-a-gang
    /// autoload still `Pending` — the shell seeds it from the resolved
    /// [`GangRegistry`](gdtf_battle_sim::ganger::GangRegistry) on the first
    /// Gang-mode frame (or marks it done when no gangs are loaded).
    fn default() -> Self {
        Self {
            name:     String::new(),
            members:  Vec::new(),
            autoload: AutoloadState::Pending,
        }
    }
}

//! The ARMOR-mode form's **working model** (GTW-479): the state-scoped [`ArmorDraft`]
//! resource the egui form's controls write and the save reads.
//!
//! The draft holds the armor's NAME (a text-field buffer — the file stem IS the registry
//! key, the GTW-269 stem-key model) plus the sim's own [`ArmorSpec`] record, so the
//! edited model IS the loader schema — projecting to the save is a copy, never a
//! parallel schema (the GTW-636 gang-draft precedent: holding the sim type means every
//! authored field round-trips by construction).

use bevy::prelude::*;
use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection, ArmorSpec,
    ArmorType, BodyPart,
};

/// The all-zero seed piece a fresh draft stamps onto every body part — the documented
/// spawn-seed-sentinel shape (`ArmorPiece::default()`, spelled `const` so the draft's
/// constructors stay `const` like [`GangDraft::new_gang`](crate::gang_form::GangDraft::new_gang)).
/// The author fills real stats in; the save writes whatever the form holds.
const SEED_PIECE: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// Whether the Armor mode's ONE-SHOT open-with-an-armor seed has run yet (GTW-479).
///
/// The Gang mode opens with the FIRST gang (sorted by name) already loaded (GTW-636);
/// the Armor mode keeps that parity via a one-shot autoload the shell runs on the first
/// Armor-mode frame. A closed enum (no-bare-types — a lifecycle phase is a domain value,
/// not a bare `bool`): [`Pending`](AutoloadState::Pending) until the shell has seen a
/// resolved [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry), then
/// [`Done`](AutoloadState::Done) forever (a "New armor" press must never be clobbered by
/// a late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    /// The one-shot registry seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The in-progress ARMOR-mode authoring DRAFT — the state-scoped resource the egui
/// form's controls write and the save projects into the loader's `(`[`ArmorName`]`,
/// `[`ArmorSpec`]`)` pair (GTW-479 C1/C2).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). The name is a bare [`String`] only as the text-field buffer (the
/// [`GangDraft`](crate::gang_form::GangDraft) `name` exception); it folds into an
/// [`ArmorName`] on projection. The suit is the sim's own [`ArmorSpec`] record (see the
/// module docs). Private fields with named accessors / mutators (no-bare-types rule 5):
/// the suit lifecycle (load / new) is owned here, while per-piece stat edits go through
/// the sim record's own public fields via [`piece_mut`](ArmorDraft::piece_mut).
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct ArmorDraft {
    /// The armor's NAME buffer — the registry key / file stem the save sanitizes.
    name:     String,
    /// The armor's editable suit — the loader-schema record itself.
    spec:     ArmorSpec,
    /// The one-shot open-with-an-armor seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl ArmorDraft {
    /// A fresh draft for a NEW armor suit: an empty name, every piece at the spawn-seed
    /// default (the author fills real stats in) — the "New armor" press. Autoload is
    /// `Done`: a deliberate new suit must never be clobbered by the one-shot registry
    /// seed (the [`GangDraft::new_gang`](crate::gang_form::GangDraft::new_gang) parity).
    #[must_use]
    pub const fn new_armor() -> Self {
        Self {
            name:     String::new(),
            spec:     ArmorSpec::uniform(SEED_PIECE),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-an-armor seed is still pending — the shell checks
    /// this each Armor-mode frame and runs the autoload exactly once (idempotent under
    /// the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the Gang mode's "no gangs loaded — start empty" parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing armor suit into the form (the load `ComboBox` / autoload path):
    /// the registry KEY becomes the name buffer and the spec is copied in as the working
    /// suit. Marks the one-shot seed done.
    pub fn load_armor(&mut self, name: &ArmorName, spec: &ArmorSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = *spec;
        self.autoload = AutoloadState::Done;
    }

    /// The armor's current NAME buffer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the armor's name (committed from the text field).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// The armor's current suit (read-only — the save projection).
    #[must_use]
    pub const fn spec(&self) -> &ArmorSpec {
        &self.spec
    }

    /// The [`ArmorPiece`] protecting `part`, mutably — per-stat edits go through the sim
    /// record's own public fields (floor / protection / integrity / hardness /
    /// `armor_type`), while the suit lifecycle stays owned by
    /// [`new_armor`](ArmorDraft::new_armor) / [`load_armor`](ArmorDraft::load_armor)
    /// (the [`GangDraft::members_mut`](crate::gang_form::GangDraft::members_mut) split).
    #[must_use]
    pub const fn piece_mut(&mut self, part: BodyPart) -> &mut ArmorPiece {
        match part {
            BodyPart::Head => &mut self.spec.head,
            BodyPart::Torso => &mut self.spec.torso,
            BodyPart::LeftArm => &mut self.spec.left_arm,
            BodyPart::RightArm => &mut self.spec.right_arm,
            BodyPart::LeftLeg => &mut self.spec.left_leg,
            BodyPart::RightLeg => &mut self.spec.right_leg,
        }
    }
}

impl Default for ArmorDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot open-with-an-armor
    /// autoload still `Pending` — the shell seeds it from the resolved
    /// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) on the first Armor-mode
    /// frame (or marks it done when no armor is loaded).
    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     ArmorSpec::uniform(SEED_PIECE),
            autoload: AutoloadState::Pending,
        }
    }
}

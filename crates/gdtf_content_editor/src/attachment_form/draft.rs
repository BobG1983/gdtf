//! The ATTACHMENT-mode form's **working model** (GTW-669): the state-scoped
//! [`AttachmentDraft`] resource the egui form's controls write and the save reads.
//!
//! The draft holds the item's NAME (a text-field buffer — the file stem IS the registry
//! key, the GTW-549 stem-key model) plus the sim's own
//! [`AttachmentSpec`] record, so the edited model IS the loader schema — projecting to
//! the save is a copy, never a parallel schema (the GTW-636 gang-draft precedent:
//! holding the loader type means every authored field round-trips by construction).
//! Field edits flow through [`spec_mut`](AttachmentDraft::spec_mut) — the one field-edit
//! seam (the `InjuryDraft::def_mut` parity); an EMPTY `effects` list is LEGAL (the
//! schema's cosmetic identity — GTW-669 C2: no `effects >= 1` floor exists to enforce).

use bevy::prelude::*;
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentSlot, AttachmentSpec},
    weapon::WeaponName,
};

/// The empty spec a fresh draft seeds — an empty display name (the author types the real
/// one), the palette's FIRST declared slot ([`Muzzle`](AttachmentSlot::Muzzle) — a
/// deliberately inert starting pick, not a design claim: the `slot` field is REQUIRED by
/// the schema so a seed must choose one), and NO effects (the schema's legal cosmetic
/// identity). `const` so [`AttachmentDraft::new_attachment`] stays `const` (the
/// `ArmorDraft::new_armor` / `SpriteDraft::new_sprite` parity).
const fn seed_spec() -> AttachmentSpec {
    AttachmentSpec {
        display_name: WeaponName::new(String::new()),
        slot:         AttachmentSlot::Muzzle,
        effects:      Vec::new(),
    }
}

/// Whether the Attachment mode's ONE-SHOT open-with-an-item seed has run yet (GTW-669).
///
/// The Gang / Armor / Injury / Sprite modes open with the FIRST member (sorted by key)
/// already loaded; the Attachment mode keeps that parity via a one-shot autoload the
/// shell runs on the first Attachment-mode frame. A closed enum (no-bare-types — a
/// lifecycle phase is a domain value, not a bare `bool`):
/// [`Pending`](AutoloadState::Pending) until the shell has seen a resolved
/// [`AttachmentRegistry`](gdtf_battle_sim::equipment::attachments::AttachmentRegistry),
/// then [`Done`](AutoloadState::Done) forever (a "New attachment" press must never be
/// clobbered by a late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    /// The one-shot registry seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The in-progress ATTACHMENT-mode authoring DRAFT — the state-scoped resource the egui
/// form's controls write and the save projects into the loader's `(`[`AttachmentName`]`,
/// `[`AttachmentSpec`]`)` pair (GTW-669 C2/C3).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). The name is a bare [`String`] only as the text-field buffer (the
/// documented `GangDraft` `name` exception); it folds into an [`AttachmentName`] on
/// projection. The spec is the sim's own [`AttachmentSpec`] record (see the module
/// docs). Private fields with named accessors / mutators (no-bare-types rule 5);
/// [`spec_mut`](AttachmentDraft::spec_mut) is the one field-edit seam — the effects list
/// is edited directly through the sim `Vec` (the `InjuryDraft` effects precedent). NOT
/// `Eq`: several effect payloads carry an `f32`.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct AttachmentDraft {
    /// The item's NAME buffer — the registry key / file stem the save sanitizes.
    name:     String,
    /// The item's editable spec — the loader-schema record itself.
    spec:     AttachmentSpec,
    /// The one-shot open-with-an-item seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl AttachmentDraft {
    /// A fresh draft for a NEW attachment: an empty name, the empty `seed_spec` (the
    /// author fills the real fields in) — the "New attachment" press. Autoload is
    /// `Done`: a deliberate new item must never be clobbered by the one-shot registry
    /// seed (the `ArmorDraft::new_armor` parity).
    #[must_use]
    pub const fn new_attachment() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-an-item seed is still pending — the shell checks
    /// this each Attachment-mode frame and runs the autoload exactly once (idempotent
    /// under the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the Gang / Armor / Sprite modes' "nothing loaded — start empty" parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing attachment item into the form (the load `ComboBox` / autoload
    /// path): the registry KEY becomes the name buffer and the spec is copied in
    /// VERBATIM as the working record — an authored file is the author's truth. Marks
    /// the one-shot seed done.
    pub fn load_attachment(&mut self, name: &AttachmentName, spec: &AttachmentSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = spec.clone();
        self.autoload = AutoloadState::Done;
    }

    /// The item's current NAME buffer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the item's name (committed from the text field).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// The item's current spec (read-only — the form's render source and the save
    /// projection).
    #[must_use]
    pub const fn spec(&self) -> &AttachmentSpec {
        &self.spec
    }

    /// The ONE field-edit seam (the `InjuryDraft::def_mut` parity): the form's controls
    /// write the display name / slot / effects list directly through the sim record, so
    /// the edited model IS the loader schema by construction.
    pub const fn spec_mut(&mut self) -> &mut AttachmentSpec {
        &mut self.spec
    }
}

impl Default for AttachmentDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot open-with-an-item
    /// autoload still `Pending` — the shell seeds it from the resolved
    /// [`AttachmentRegistry`](gdtf_battle_sim::equipment::attachments::AttachmentRegistry)
    /// on the first Attachment-mode frame (or marks it done when no items are loaded).
    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}

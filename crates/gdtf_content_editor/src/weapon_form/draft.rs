//! The WEAPON-mode form's **working model** (GTW-670): the state-scoped [`WeaponDraft`]
//! resource the egui form's controls write and the save reads.
//!
//! The draft holds the weapon's NAME (a text-field buffer — the file stem IS the
//! registry key, the GTW-257 stem-key model) plus the sim's own [`WeaponSpec`] record,
//! so the edited model IS the loader schema — projecting to the save is a copy, never a
//! parallel schema (the GTW-636 gang-draft precedent: holding the loader type means
//! every authored field round-trips by construction). Field edits flow through
//! [`spec_mut`](WeaponDraft::spec_mut) — the one field-edit seam (the
//! `AttachmentDraft::spec_mut` parity); the list fields whose sim newtypes are
//! construct-only ([`FireMode`] / [`WeaponSlots`](gdtf_battle_sim::equipment::attachments::WeaponSlots))
//! are edited by projecting the authored list out, mutating it, and folding it back
//! through the SAME constructor the loader's deserialize uses.

use std::num::NonZeroU8;

use bevy::prelude::*;
use gdtf_battle_sim::{
    equipment::attachments::WeaponSlots,
    magazine::Magazine,
    weapon::{
        Accuracy, BaseSpread, DamageType, DotTurns, FatalBias, FireMode, FireModeSpec, Handedness,
        Kickback, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Shove, Stable, TrajectoryStyle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WeaponSpec,
    },
};

/// The empty spec a fresh draft seeds — every scalar at its zero magnitude, the
/// documented vocabulary defaults ([`DamageType::Kinetic`] / [`Handedness::OneHanded`] /
/// [`TrajectoryStyle::Straight`]), an empty magazine, ONE structural single-shot fire
/// mode (the [`FireMode`] invariant: "a well-authored weapon lists at least one mode,
/// with `Single` first" — the same structural mode [`FireMode::single`] falls back to),
/// and the serde-default identities for every opt-in field (no slots / attachments /
/// dot / `on_death`; `shove` off). Starting points the author immediately re-tunes,
/// never design claims.
fn seed_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy:    Accuracy::new(0.0),
        kickback:    Kickback::new(0.0),
        fatal_bias:  FatalBias::new(0.0),
        damage:      WeaponDamage::new(0),
        punch:       WeaponPunch::new(0),
        shred:       WeaponShred::new(0),
        damage_type: DamageType::Kinetic,
        magazine:    Magazine::default(),
        fire_mode:   FireMode::new(vec![structural_single_mode()]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Straight,
        slots:       WeaponSlots::default(),
        attachments: Vec::new(),
        dot:         None,
        on_death:    None,
    }
}

/// The sim's structural single-shot fire mode (`Single`, cone ×1.0, 0% TU, 1 shot) —
/// the [`FireMode::single`] total-fallback shape, reused as the seed row a fresh weapon
/// starts with and the template an Add-mode press appends (the GTW-669
/// `fire_mode_edit` `GainFireMode` template verbatim).
#[must_use]
pub(crate) const fn structural_single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.0),
        ModeShots::new(1),
    )
}

/// Fold a raw drag value into a [`DotTurns`] with the GTW-643 floor: `0` becomes the
/// minimum ONE turn — an authored zero-turn DOT is unrepresentable by type
/// ([`DotTurns`] wraps [`NonZeroU8`]), so the form's commit path keeps it unauthorable
/// too (GTW-670 C2: the drag clamps to `1..=u8::MAX` and this fold backstops it).
#[must_use]
pub(crate) fn dot_turns_from_raw(raw: u8) -> DotTurns {
    DotTurns::new(NonZeroU8::new(raw).unwrap_or(NonZeroU8::MIN))
}

/// Whether the Weapon mode's ONE-SHOT open-with-a-weapon seed has run yet (GTW-670).
///
/// The Gang / Armor / Injury / Sprite / Attachment modes open with the FIRST member
/// (sorted by key) already loaded; the Weapon mode keeps that parity via a one-shot
/// autoload the shell runs on the first Weapon-mode frame. A closed enum
/// (no-bare-types — a lifecycle phase is a domain value, not a bare `bool`):
/// [`Pending`](AutoloadState::Pending) until the shell has seen a resolved
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry), then
/// [`Done`](AutoloadState::Done) forever (a "New weapon" press must never be clobbered
/// by a late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    /// The one-shot registry seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The in-progress WEAPON-mode authoring DRAFT — the state-scoped resource the egui
/// form's controls write and the save projects into the loader's `(`[`WeaponName`]`,
/// `[`WeaponSpec`]`)` pair (GTW-670 C2/C3).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)`
/// — bevy-traps #1). The name is a bare [`String`] only as the text-field buffer (the
/// documented `GangDraft` `name` exception); it folds into a [`WeaponName`] on
/// projection. The spec is the sim's own [`WeaponSpec`] record (see the module docs).
/// Private fields with named accessors / mutators (no-bare-types rule 5);
/// [`spec_mut`](WeaponDraft::spec_mut) is the one field-edit seam. NOT `Eq`: several
/// spec fields carry an `f32`.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct WeaponDraft {
    /// The weapon's NAME buffer — the registry key / file stem the save sanitizes.
    name:     String,
    /// The weapon's editable spec — the loader-schema record itself.
    spec:     WeaponSpec,
    /// The one-shot open-with-a-weapon seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl WeaponDraft {
    /// A fresh draft for a NEW weapon: an empty name, the empty `seed_spec` (the author
    /// fills the real fields in) — the "New weapon" press. Autoload is `Done`: a
    /// deliberate new weapon must never be clobbered by the one-shot registry seed (the
    /// `AttachmentDraft::new_attachment` parity).
    #[must_use]
    pub fn new_weapon() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-a-weapon seed is still pending — the shell checks
    /// this each Weapon-mode frame and runs the autoload exactly once (idempotent under
    /// the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the Gang / Armor / Attachment modes' "nothing loaded — start empty"
    /// parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing weapon into the form (the load `ComboBox` / autoload path): the
    /// registry KEY becomes the name buffer and the spec is copied in VERBATIM as the
    /// working record — an authored file is the author's truth. Marks the one-shot seed
    /// done.
    pub fn load_weapon(&mut self, name: &WeaponName, spec: &WeaponSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = spec.clone();
        self.autoload = AutoloadState::Done;
    }

    /// The weapon's current NAME buffer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the weapon's name (committed from the text field).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// The weapon's current spec (read-only — the form's render source and the save
    /// projection).
    #[must_use]
    pub const fn spec(&self) -> &WeaponSpec {
        &self.spec
    }

    /// The ONE field-edit seam (the `AttachmentDraft::spec_mut` parity): the form's
    /// controls write every authored field directly through the sim record, so the
    /// edited model IS the loader schema by construction.
    pub const fn spec_mut(&mut self) -> &mut WeaponSpec {
        &mut self.spec
    }
}

impl Default for WeaponDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot open-with-a-weapon
    /// autoload still `Pending` — the shell seeds it from the resolved
    /// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) on the first
    /// Weapon-mode frame (or marks it done when no weapons are loaded).
    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}

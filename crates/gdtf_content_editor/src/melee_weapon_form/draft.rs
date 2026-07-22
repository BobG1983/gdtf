//! The MELEE-WEAPON-mode form's **working model** (GTW-671): the state-scoped
//! [`MeleeWeaponDraft`] resource the egui form's controls write and the save reads.
//!
//! The draft holds the weapon's NAME (a text-field buffer — the file stem IS the
//! registry key, the GTW-257 stem-key model shared by the melee family) plus the sim's
//! own [`MeleeWeaponSpec`] record, so the edited model IS the loader schema — projecting
//! to the save is a copy, never a parallel schema (the GTW-670 `WeaponDraft` precedent).
//! Field edits flow through [`spec_mut`](MeleeWeaponDraft::spec_mut) — the one
//! field-edit path; the list fields whose sim newtypes are construct-only
//! ([`FightMode`] / [`WeaponSlots`](gdtf_battle_sim::equipment::attachments::WeaponSlots))
//! are edited by projecting the authored list out, mutating it, and folding it back
//! through the SAME constructor the loader's deserialize uses.

use bevy::prelude::*;
use gdtf_battle_sim::{
    equipment::attachments::WeaponSlots,
    weapon::{
        DamageType, FatalBias, FightMode, FightModeKind, FightModeSpec, Handedness,
        MeleeWeaponSpec, Reach, Shove, Strikes, TuCost, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

/// The empty spec a fresh draft seeds — every damage scalar at its zero magnitude, the
/// documented vocabulary defaults ([`DamageType::Kinetic`] /
/// [`Handedness::OneHanded`]), the [`Reach::DEFAULT`] adjacent-cell reach, ONE
/// structural swing fight mode (the [`FightMode`] invariant: "a well-authored melee
/// weapon lists at least one mode" — the same structural mode
/// [`FightMode::primary`] falls back to), and the serde-default identities for every
/// opt-in field (no slots / attachments; `shove` off). Starting points the author
/// immediately re-tunes, never design claims.
fn seed_spec() -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage:      WeaponDamage::new(0),
        punch:       WeaponPunch::new(0),
        shred:       WeaponShred::new(0),
        damage_type: DamageType::Kinetic,
        fatal_bias:  FatalBias::new(0.0),
        handedness:  Handedness::OneHanded,
        reach:       Reach::DEFAULT,
        fight_mode:  FightMode::new(vec![structural_swing_mode()]),
        shove:       Shove::new(false),
        slots:       WeaponSlots::default(),
        attachments: Vec::new(),
    }
}

/// The sim's structural single-swing fight mode (`Swing`, 0 TU, 1 strike) — the
/// [`FightMode::primary`] total-fallback shape, reused as the seed row a fresh melee
/// weapon starts with and the template an Add-mode press appends (the GTW-670
/// `structural_single_mode` parity).
#[must_use]
pub(crate) const fn structural_swing_mode() -> FightModeSpec {
    FightModeSpec::new(FightModeKind::Swing, TuCost::new(0), Strikes::new(1))
}

/// Whether the MELEE-WEAPON mode's ONE-SHOT open-with-a-weapon seed has run yet
/// (GTW-671).
///
/// The Gang / Armor / Injury / Sprite / Attachment / Weapon modes open with the FIRST
/// member (sorted by key) already loaded; the MELEE mode keeps that parity via a
/// one-shot autoload the shell runs on the first MeleeWeapon-mode frame. A closed enum
/// (no-bare-types — a lifecycle phase is a domain value, not a bare `bool`):
/// [`Pending`](AutoloadState::Pending) until the shell has seen a resolved
/// [`MeleeWeaponRegistry`](gdtf_battle_sim::weapon::MeleeWeaponRegistry), then
/// [`Done`](AutoloadState::Done) forever (a "New melee weapon" press must never be
/// clobbered by a late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    /// The one-shot registry seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The in-progress MELEE-WEAPON-mode authoring DRAFT — the state-scoped resource the
/// egui form's controls write and the save projects into the loader's
/// `(`[`WeaponName`]`, `[`MeleeWeaponSpec`]`)` pair (GTW-671 C2/C3).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)`
/// — bevy-traps #1). The name is a bare [`String`] only as the text-field buffer (the
/// documented `GangDraft` `name` exception); it folds into a [`WeaponName`] on
/// projection. The spec is the sim's own [`MeleeWeaponSpec`] record (see the module
/// docs). Private fields with named accessors / mutators (no-bare-types rule 5);
/// [`spec_mut`](MeleeWeaponDraft::spec_mut) is the one field-edit path. NOT `Eq`: the
/// spec's `fatal_bias` carries an `f32`.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct MeleeWeaponDraft {
    /// The weapon's NAME buffer — the registry key / file stem the save sanitizes.
    name:     String,
    /// The weapon's editable spec — the loader-schema record itself.
    spec:     MeleeWeaponSpec,
    /// The one-shot open-with-a-weapon seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl MeleeWeaponDraft {
    /// A fresh draft for a NEW melee weapon: an empty name, the empty `seed_spec` (the
    /// author fills the real fields in) — the "New melee weapon" press. Autoload is
    /// `Done`: a deliberate new weapon must never be clobbered by the one-shot registry
    /// seed (the `WeaponDraft::new_weapon` parity).
    #[must_use]
    pub fn new_melee_weapon() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-a-weapon seed is still pending — the shell checks
    /// this each MeleeWeapon-mode frame and runs the autoload exactly once (idempotent
    /// under the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the Gang / Armor / Weapon modes' "nothing loaded — start empty" parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing melee weapon into the form (the load `ComboBox` / autoload
    /// path): the registry KEY becomes the name buffer and the spec is copied in
    /// VERBATIM as the working record — an authored file is the author's truth. Marks
    /// the one-shot seed done.
    pub fn load_melee_weapon(&mut self, name: &WeaponName, spec: &MeleeWeaponSpec) {
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
    pub const fn spec(&self) -> &MeleeWeaponSpec {
        &self.spec
    }

    /// The ONE field-edit path (the `WeaponDraft::spec_mut` parity): the form's
    /// controls write every authored field directly through the sim record, so the
    /// edited model IS the loader schema by construction.
    pub const fn spec_mut(&mut self) -> &mut MeleeWeaponSpec {
        &mut self.spec
    }
}

impl Default for MeleeWeaponDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot open-with-a-weapon
    /// autoload still `Pending` — the shell seeds it from the resolved
    /// [`MeleeWeaponRegistry`](gdtf_battle_sim::weapon::MeleeWeaponRegistry) on the
    /// first MeleeWeapon-mode frame (or marks it done when no melee weapons are
    /// loaded).
    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}

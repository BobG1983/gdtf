//! The **gang** roster model — the GTW-414/GTW-415 faction-agnostic reusable roster.
//!
//! A *gang* (glossary: a *Gang* is a faction / the player's roster as a unit) is a
//! NAMED list of roster members, each carrying identity + the eight direct attributes +
//! a weapon KEY + an armor KEY — and NOTHING about WHERE it fights or WHICH side it is
//! on. Placement (`at` / `facing` / `stance` / `aiming` / `life_state`) and faction
//! assignment live on the [`Situation`](crate::situation::Situation) side
//! ([`PlacedGanger`](crate::situation::PlacedGanger)), so the SAME gang roster can be
//! fielded in any battle, on any side, at any position — the roster is reusable, the
//! situation assigns the rest.
//!
//! The pieces:
//!
//! - [`GangName`] — the gang's registry KEY (= its `*.gang.ron` filename stem). Defined
//!   HERE (GTW-414) and consumed by the GTW-415 loader as the [`GangRegistry`] key.
//! - [`GangMember`] — one roster member: a [`GangerName`] identity, the eight direct
//!   attributes, a [`WeaponName`] key, an [`ArmorName`] key. NO faction, NO placement.
//! - [`GangRoster`] — the gang ASSET: a `Vec<GangMember>`. The registry key (the gang
//!   name) is the file stem, NOT a field of the asset (mirroring the weapon/armor key
//!   model — the file's name IS the key).
//! - [`GangRegistry`] — the name→roster [`Resource`] the `Load` folder loader builds and
//!   [`setup_battle`](crate::situation::setup_battle) resolves
//!   [`PlacedGanger`](crate::situation::PlacedGanger) gang/member refs against.

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
    reflect::TypePath,
};
use serde::Deserialize;

use super::{
    attributes::{Aim, Cool, Grit, Reflexes, Speed, Strength},
    vitals::{GangerName, Luck, Toughness},
};
use crate::{armor::ArmorName, weapon::WeaponName};

/// A **gang** identity — the name of a faction-agnostic reusable roster (GTW-414).
///
/// The gang's registry KEY: the filename stem (without the `.gang` infix) of an
/// `assets/content/gangs/*.gang.ron`, so `goliaths.gang.ron` keys gang `"goliaths"`.
/// [`PlacedGanger`](crate::situation::PlacedGanger) references its gang by this name,
/// resolved against the [`GangRegistry`] at
/// [`setup_battle`](crate::situation::setup_battle) into the roster the placed ganger's
/// identity / attributes / equipment come from.
///
/// A name newtype over [`String`] (no-bare-types: a name is a domain value, not a bare
/// `String`), the [`GangerName`] / [`WeaponName`] precedent. Private inner + derived
/// [`Deref`] (house style — never a hand-written `impl Deref`). `#[serde(transparent)]`
/// lets an authored situation `.ron`'s `gang` ref parse as a bare string. `Eq` + `Hash`
/// so it keys the [`GangRegistry`]'s [`HashMap`].
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct GangName(String);

impl GangName {
    /// Build a gang name from its key string — the public constructor (house style) so
    /// the loader (keying by file stem) and tests can build a `GangName` without
    /// reaching the private field.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One **gang roster member** — a faction-agnostic, placement-free roster entry
/// (GTW-415).
///
/// The roster half of the old combined `GangerSpawn`: a [`GangerName`] identity, the
/// EIGHT direct attributes ([`Speed`] / [`Aim`] / [`Strength`] / [`Toughness`] /
/// [`Reflexes`] / [`Cool`] / [`Grit`] / [`Luck`] — the raw authored potential, GTW-384),
/// a weapon KEY ([`WeaponName`]) and an armor KEY ([`ArmorName`]). It carries NO
/// [`Faction`](crate::ganger::Faction) and NO placement — those are assigned per-battle
/// by the [`Situation`](crate::situation::Situation), so one gang roster is reusable on
/// any side at any position.
///
/// [`setup_battle`](crate::situation::setup_battle) resolves a
/// [`PlacedGanger`](crate::situation::PlacedGanger)'s `(gang, member)` pair to this
/// record, then DERIVES the computed combat stats from the eight attributes ×
/// [`GangerStatTuning`](crate::tuning::GangerStatTuning) and resolves the weapon / armor
/// keys against the [`WeaponRegistry`](crate::weapon::WeaponRegistry) /
/// [`ArmorRegistry`](crate::armor::ArmorRegistry) (mirroring the old `GangerSpawn` path).
///
/// Not `Eq` / `Hash` / `Copy`: the attributes carry `f32` magnitudes (no total order),
/// and the name / weapon / armor keys are owned [`String`]-backed newtypes — so a member
/// is `Clone` + `PartialEq` only. Derives [`Deserialize`] so an authored
/// `*.gang.ron` names each member's identity + eight attributes + weapon + armor as a
/// self-describing record (the whole value graph flows through the landed newtype serde
/// derives — render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GangMember {
    /// The member's **name** — its human-facing display identity (the
    /// [`PlacedGanger`](crate::situation::PlacedGanger) references it by this name to
    /// pick a member out of the gang). Authored as a bare string ([`GangerName`] is
    /// `#[serde(transparent)]`).
    pub name:      GangerName,
    /// The member's **Speed** direct attribute — quickness (GTW-384). Drives the derived
    /// [`Tu`](crate::ganger::Tu) budget + Fight/Reactions terms. Authored as a bare
    /// scalar ([`Speed`] is `#[serde(transparent)]`).
    pub speed:     Speed,
    /// The member's **Aim** direct attribute — innate marksmanship (GTW-384). The
    /// dominant derived [`Shooting`](crate::ganger::Shooting) term. Authored as a bare
    /// scalar.
    pub aim:       Aim,
    /// The member's **Strength** direct attribute — physical power (GTW-384). A derived
    /// Fight term. Authored as a bare scalar.
    pub strength:  Strength,
    /// The member's **Toughness** direct attribute — damage resistance. A (reused)
    /// severity-roll term + a derived [`Hp`](crate::ganger::Hp) term. Authored as a bare
    /// scalar.
    pub toughness: Toughness,
    /// The member's **Reflexes** direct attribute — reaction speed (GTW-384). A derived
    /// Shooting + Reactions term. Authored as a bare scalar.
    pub reflexes:  Reflexes,
    /// The member's **Cool** direct attribute — nerves under fire (GTW-384). The broad
    /// Shooting/Fight/Reactions/HP/Morale contributor. Authored as a bare scalar.
    pub cool:      Cool,
    /// The member's **Grit** direct attribute — resilience (GTW-384). The dominant
    /// derived [`Hp`](crate::ganger::Hp) + Morale term. Authored as a bare scalar.
    pub grit:      Grit,
    /// The member's **Luck** direct attribute — directional fortune. The (reused)
    /// severity-roll tail (feeds the severity roll ONLY, never the computed stats). Authored
    /// as a bare scalar.
    pub luck:      Luck,
    /// The member's **armor KEY** — the filename stem of an
    /// `assets/content/armor/*.armor.ron`, resolved against the
    /// [`ArmorRegistry`](crate::armor::ArmorRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle).
    pub armor:     ArmorName,
    /// The member's **weapon KEY** — the filename stem of an
    /// `assets/content/weapons/*.weapon.ron`, resolved against the
    /// [`WeaponRegistry`](crate::weapon::WeaponRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle).
    pub weapon:    WeaponName,
}

impl GangMember {
    /// Look up the member's eight direct attributes as a
    /// [`GangerAttributes`](crate::ganger::GangerAttributes) record — the input
    /// [`derive_stats`](crate::ganger::derive_stats) computes the member's combat stats
    /// from at setup.
    #[must_use]
    pub const fn attributes(&self) -> crate::ganger::GangerAttributes {
        crate::ganger::GangerAttributes {
            speed:     self.speed,
            aim:       self.aim,
            strength:  self.strength,
            toughness: self.toughness,
            reflexes:  self.reflexes,
            cool:      self.cool,
            grit:      self.grit,
            luck:      self.luck,
        }
    }
}

/// A **gang roster** — the `*.gang.ron` ASSET: a list of [`GangMember`]s (GTW-415).
///
/// The gang's name is NOT a field — it is the registry KEY (the file stem), exactly as a
/// weapon's name is its file stem, never a field of the [`WeaponSpec`](crate::weapon::WeaponSpec).
/// So a roster asset is purely its member list; the [`GangRegistry`] keys it by the
/// loaded file's stem.
///
/// Derives [`Deserialize`] (an authored `*.gang.ron` is a `(members: [ … ])` record) +
/// [`TypePath`] (render-free reflection metadata, no rendering) because the
/// `RonAsset<GangRoster>` the GTW-415 folder loader wraps it in requires its payload to
/// be [`TypePath`] — the same bound [`WeaponSpec`](crate::weapon::WeaponSpec) /
/// [`Situation`](crate::situation::Situation) satisfy. `#[serde(default)]` on `members`
/// lets an authored file omit an empty member list.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TypePath)]
pub struct GangRoster {
    /// The gang's roster members (each a [`GangMember`] — identity + eight attributes +
    /// weapon + armor keys). `#[serde(default)]` gives an empty roster for a file that
    /// omits the field.
    #[serde(default)]
    pub members: Vec<GangMember>,
}

impl GangRoster {
    /// Build a roster from its member list — the shape a test (and the loader) builds.
    #[must_use]
    pub fn new(members: impl IntoIterator<Item = GangMember>) -> Self {
        Self {
            members: members.into_iter().collect(),
        }
    }

    /// Look up a member by its [`GangerName`], or [`None`] if no member of this gang
    /// carries that name — the per-member resolution
    /// [`setup_battle`](crate::situation::setup_battle) runs for each
    /// [`PlacedGanger`](crate::situation::PlacedGanger).
    #[must_use]
    pub fn member(&self, name: &GangerName) -> Option<&GangMember> {
        self.members.iter().find(|member| &member.name == name)
    }
}

/// The **gang registry** — a name→roster map the `Load` folder loader builds and
/// [`setup_battle`](crate::situation::setup_battle) resolves
/// [`PlacedGanger`](crate::situation::PlacedGanger) gang refs against (GTW-415).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`GangName`]`, `[`GangRoster`]`>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), the
/// [`WeaponRegistry`](crate::weapon::WeaponRegistry) precedent. The sim OWNS the gang
/// model, so the type lives here; the app's `Load` flow POPULATES it from the loaded
/// `assets/content/gangs/*.gang.ron` folder (keyed by each file's stem) and inserts it as
/// a resource. It holds the rosters BY VALUE ([`GangRoster`] is `Clone`), so they survive
/// the loaded-folder asset handle being dropped on `OnExit(Load)`.
///
/// Private inner with small accessors (the registry answers a roster LOOKUP, not a
/// raw-map question — so no derived [`Deref`]). [`setup_battle`](crate::situation::setup_battle)
/// resolves a gang ref through [`roster`](GangRegistry::roster).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct GangRegistry(HashMap<GangName, GangRoster>);

impl GangRegistry {
    /// Build a gang registry from a `(name, roster)` iterator — the shape the folder
    /// loader (keyed by filename stem) and a test build.
    #[must_use]
    pub fn new(gangs: impl IntoIterator<Item = (GangName, GangRoster)>) -> Self {
        Self(gangs.into_iter().collect())
    }

    /// Insert one roster under its [`GangName`] key, returning the previous roster at
    /// that key (if any) — the per-file insert the folder loader calls as it iterates the
    /// loaded folder.
    pub fn insert(&mut self, name: GangName, roster: GangRoster) -> Option<GangRoster> {
        self.0.insert(name, roster)
    }

    /// Look up the [`GangRoster`] for a gang KEY, or [`None`] if no gang file with that
    /// stem was loaded — the setup-time resolution the battle reads.
    #[must_use]
    pub fn roster(&self, name: &GangName) -> Option<&GangRoster> {
        self.0.get(name)
    }

    /// How many gangs the registry holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no gangs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over every [`GangName`] key in the registry — for editor / roster
    /// enumeration (the [`WeaponRegistry::keys`](crate::weapon::WeaponRegistry) precedent),
    /// so a roster UI can list every loaded gang without exposing the inner map.
    ///
    /// [`HashMap`] iteration order is unspecified; callers that need a stable order must
    /// collect and sort.
    pub fn keys(&self) -> impl Iterator<Item = &GangName> {
        self.0.keys()
    }
}

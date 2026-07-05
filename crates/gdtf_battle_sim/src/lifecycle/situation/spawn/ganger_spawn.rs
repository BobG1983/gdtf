//! [`GangerSpawn`] — the combined roster-plus-placement authoring helper and its
//! GTW-414 schema-v2 [`split`](GangerSpawn::split) into a
//! [`PlacedGanger`] + [`GangMember`](crate::ganger::GangMember) pair.

use serde::Deserialize;

use super::placed_ganger::{PlacedGanger, Placement};
use crate::{
    ganger::{
        Aim, Aiming, Cool, Facing, Faction, GangMember, GangName, GangerName, Grit, LifeState,
        Luck, Reflexes, Speed, Stance, Strength, Toughness,
    },
    metric::CellLevel,
    weapon::WeaponName,
};

/// A combined **roster-plus-placement** authoring record — the GTW-414 schema-v2
/// authoring HELPER that the test builders and the migration assemble, then SPLIT into a
/// [`PlacedGanger`] (the `Situation`-side placement + faction) and a
/// [`GangMember`](crate::ganger::GangMember) (the gang-roster identity + attributes +
/// equipment) via [`split`](GangerSpawn::split).
///
/// Before GTW-414 this WAS the `Situation.gangers` element (roster + placement + faction
/// in one struct). The v2 schema splits roster (the reusable, faction-agnostic gang
/// member) from placement (where + which side the situation assigns), so the canonical
/// `Situation` now holds [`PlacedGanger`]s and the gang rosters live in a
/// [`GangRegistry`](crate::ganger::GangRegistry). This combined record survives ONLY as
/// the ergonomic authoring shape (one literal carries everything about one fielded
/// ganger), and [`split`](GangerSpawn::split) decomposes it into the two v2 halves.
///
/// A named struct (not a bare tuple) so the authored ganger shape is self-describing.
/// Since GTW-384 the situation authors the EIGHT DIRECT ATTRIBUTES
/// ([`Speed`] / [`Aim`] / [`Strength`] / [`Toughness`] / [`Reflexes`] / [`Cool`] /
/// [`Grit`] / [`Luck`]) — NOT the computed stats. The flat computed-stat literals it
/// used to carry (`hp` / `hp_max` / `wounds` / `wounds_max` / `tu` / `tu_max` /
/// `shooting`) are GONE: [`setup_battle`](crate::situation::setup_battle) DERIVES them
/// from the attributes × the
/// [`GangerStatTuning`](crate::tuning::GangerStatTuning) weights
/// (`docs/combat/stats.md` §"Computed combat stats", the two-layer model — fully
/// derived, single source of truth). `armor` is the armor KEY ([`ArmorName`](crate::armor::ArmorName))
/// resolved at setup against the [`ArmorRegistry`](crate::armor::ArmorRegistry) into the
/// [`ArmorSpec`](crate::armor::ArmorSpec) that the spawned ganger's battle-local
/// armor-piece entities ([`Wears`](crate::armor::Wears)) are seeded from (GTW-269 /
/// GTW-323 — mirroring the [`weapon`](GangerSpawn::weapon) key, whose resolved bundle
/// spawns the related weapon entity). The grid key [`at`](GangerSpawn::at) becomes the
/// spawned ganger's [`Position`](crate::ganger::Position).
///
/// Not `Eq` / `Hash`: the attributes carry `f32` magnitudes (no total order), so the
/// authored ganger is `PartialEq` only. `(cell, level)`-keyed de-duplication
/// ([`has_stacked_gangers`](crate::situation::has_stacked_gangers)) hashes
/// [`at`](GangerSpawn::at), never the whole struct.
///
/// Not `Copy` (GTW-257 / GTW-285 / GTW-269): the [`weapon`](GangerSpawn::weapon) key
/// is a [`WeaponName`] over a [`String`], the [`armor`](GangerSpawn::armor) key is an
/// [`ArmorName`](crate::armor::ArmorName) over a [`String`], and the
/// [`name`](GangerSpawn::name) is a [`GangerName`] over a [`String`] (all owned, not
/// `Copy`), so the authored ganger is `Clone` only. The
/// [`setup_battle`](crate::situation::setup_battle) spawn loop borrows each ganger, so
/// dropping `Copy` costs nothing on the real path.
///
/// Derives [`Deserialize`] so an authored situation `.ron` names each ganger's
/// placement + its eight attributes + roster armor + its [`weapon`](GangerSpawn::weapon)
/// key (the value graph all flows through the landed newtype/enum serde derives —
/// render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GangerSpawn {
    /// The `(cell, level)` the ganger spawns at — its [`Position`](crate::ganger::Position).
    pub at:         CellLevel,
    /// The ganger's **name** — its human-facing display identity (GTW-285).
    /// [`setup_battle`](crate::situation::setup_battle) spawns it as a [`GangerName`]
    /// component beside the rest of the per-field set; the status panel's identity line
    /// renders it (replacing the placeholder cell location). Authored per ganger in the
    /// situation `.ron` as a bare string ([`GangerName`] is `#[serde(transparent)]`).
    pub name:       GangerName,
    /// The ganger's gang (faction) identity.
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
    /// The ganger's **Speed** direct attribute — quickness (GTW-384). Drives the
    /// derived [`Tu`](crate::ganger::Tu) budget + Fight/Reactions terms. Authored as a
    /// bare scalar ([`Speed`] is `#[serde(transparent)]`).
    pub speed:      Speed,
    /// The ganger's **Aim** direct attribute — innate marksmanship (GTW-384). The
    /// dominant derived [`Shooting`](crate::ganger::Shooting) term. Authored as a bare
    /// scalar ([`Aim`] is `#[serde(transparent)]`).
    pub aim:        Aim,
    /// The ganger's **Strength** direct attribute — physical power (GTW-384). A derived
    /// Fight term. Authored as a bare scalar ([`Strength`] is `#[serde(transparent)]`).
    pub strength:   Strength,
    /// The ganger's **Toughness** direct attribute — damage resistance. REUSED (the §6
    /// severity roll already reads it); ALSO a term in the derived
    /// [`Hp`](crate::ganger::Hp) pool (GTW-384). Authored as a bare scalar.
    pub toughness:  Toughness,
    /// The ganger's **Reflexes** direct attribute — reaction speed (GTW-384). A derived
    /// Shooting + Reactions term. Authored as a bare scalar ([`Reflexes`] is
    /// `#[serde(transparent)]`).
    pub reflexes:   Reflexes,
    /// The ganger's **Cool** direct attribute — nerves under fire (GTW-384). The broad
    /// Shooting/Fight/Reactions/HP/Morale contributor. Authored as a bare scalar
    /// ([`Cool`] is `#[serde(transparent)]`).
    pub cool:       Cool,
    /// The ganger's **Grit** direct attribute — resilience (GTW-384). The dominant
    /// derived [`Hp`](crate::ganger::Hp) + Morale term. Authored as a bare scalar
    /// ([`Grit`] is `#[serde(transparent)]`).
    pub grit:       Grit,
    /// The ganger's **Luck** direct attribute — directional fortune. REUSED (the §6
    /// severity roll reads it; feeds the severity roll ONLY, never the computed stats —
    /// `docs/combat/stats.md`). Authored as a bare scalar.
    pub luck:       Luck,
    /// The ganger's **armor KEY** — the filename stem of an `assets/content/armor/*.armor.ron`
    /// (e.g. `"flak_vest"`), resolved against the
    /// [`ArmorRegistry`](crate::armor::ArmorRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the
    /// [`ArmorSpec`](crate::armor::ArmorSpec) that the spawned ganger's battle-local
    /// armor-piece entities ([`Wears`](crate::armor::Wears)) are seeded from by value —
    /// never mutated on the roster (GTW-269 / GTW-323, mirroring the
    /// [`weapon`](GangerSpawn::weapon) key). A key absent from the registry is a handled
    /// [`BattleSetupError::ArmorNotFound`](crate::situation::BattleSetupError::ArmorNotFound)
    /// error (no panic).
    pub armor:      crate::armor::ArmorName,
    /// The ganger's **weapon KEY** — the filename stem of an `assets/content/weapons/ranged/*.ron`
    /// (e.g. `"stub_pistol"`), resolved against the
    /// [`WeaponRegistry`](crate::weapon::WeaponRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the
    /// [`WeaponBundle`](crate::weapon::WeaponBundle) spawned onto the related weapon
    /// entity ([`Wields`](crate::weapon::Wields), GTW-257 / GTW-323). REQUIRED — every
    /// authored ganger is armed; an
    /// unarmed `Option<WeaponName>` case is a deliberate FUTURE option (the
    /// [[weapons-armor-data-driven]] model arms every ganger for now). A key absent
    /// from the registry is a handled
    /// [`BattleSetupError::WeaponNotFound`](crate::situation::BattleSetupError::WeaponNotFound)
    /// error (no panic).
    pub weapon:     WeaponName,
}

impl GangerSpawn {
    /// Split this combined authoring record into its GTW-414 schema-v2 halves: a
    /// [`PlacedGanger`] (the `Situation`-side placement + faction) referencing the given
    /// `gang`, paired with the [`GangMember`](crate::ganger::GangMember) (the gang-roster
    /// identity + eight attributes + weapon / armor keys) it points at.
    ///
    /// The placement's [`member`](PlacedGanger::member) is this ganger's
    /// [`name`](GangerSpawn::name), so the returned `PlacedGanger.member` resolves against
    /// the returned `GangMember` in the registry. The test builders and the migration use
    /// this to assemble a v2 [`Situation`](crate::situation::Situation) + [`GangRegistry`](crate::ganger::GangRegistry)
    /// pair from the ergonomic combined literal.
    #[must_use]
    pub fn split(&self, gang: GangName) -> (PlacedGanger, GangMember) {
        let placed = PlacedGanger::new(
            gang,
            self.name.clone(),
            Placement::new(
                self.at,
                self.faction,
                self.facing,
                self.stance,
                self.aiming,
                self.life_state,
            ),
        );
        let member = GangMember {
            name:         self.name.clone(),
            speed:        self.speed,
            aim:          self.aim,
            strength:     self.strength,
            toughness:    self.toughness,
            reflexes:     self.reflexes,
            cool:         self.cool,
            grit:         self.grit,
            luck:         self.luck,
            armor:        self.armor.clone(),
            weapon:       self.weapon.clone(),
            // GTW-505: the combined `GangerSpawn` authors no melee weapon — the split
            // member gets `None`, which `setup_battle` resolves to the shipped `fists`
            // default (every ganger can melee). A future builder method can override it.
            melee_weapon: None,
        };
        (placed, member)
    }
}

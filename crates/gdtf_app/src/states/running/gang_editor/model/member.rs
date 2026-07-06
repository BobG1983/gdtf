//! The editable MEMBER record of the gang editor's working model: its fields, the sim
//! round-trip projections, and the per-field setters. Split out of the monolithic
//! `model.rs` (GTW-583); the model rationale lives on the parent `model` module.

use bevy::prelude::*;
use gdtf_battle_sim::{
    Aim, ArmorName, Cool, GangMember, GangerAttributes, GangerName, Grit, Reflexes, Speed,
    Strength, Toughness, WeaponName, ganger::Luck,
};

use crate::states::running::gang_editor::components::BaseAttribute;

/// The default new-member display name the SCAFFOLD's "Add member" stamps onto a fresh
/// [`EditableMember`]. A named placeholder so a freshly-added row reads as a member rather
/// than blank.
const DEFAULT_MEMBER_NAME: &str = "New Member";

crate::support_item! {
    /// One **editable gang member** in the editor's working model (GTW-420).
    ///
    /// A faction-agnostic, placement-free roster entry mirroring the sim's
    /// [`GangMember`](gdtf_battle_sim::GangMember): an identity [`GangerName`], the eight direct
    /// attributes, and the weapon / armor KEY newtypes. It is a NEW editor-side type (not the sim
    /// asset record) because the editor needs an owned, mutable working copy the screen can edit.
    ///
    /// "Add member" appends [`EditableMember::default_member`] — a default record. GTW-425's
    /// per-field inline editing (name field, weapon / armor dropdowns) then mutates the member in
    /// place through the named setters below ([`set_name`](EditableMember::set_name) /
    /// [`set_weapon`](EditableMember::set_weapon) / [`set_armor`](EditableMember::set_armor)).
    #[derive(Clone, PartialEq, Debug)]
    struct EditableMember {
        /// The member's display identity ([`GangerName`]).
        name:      GangerName,
        /// The member's **Speed** direct attribute.
        speed:     Speed,
        /// The member's **Aim** direct attribute.
        aim:       Aim,
        /// The member's **Strength** direct attribute.
        strength:  Strength,
        /// The member's **Toughness** direct attribute.
        toughness: Toughness,
        /// The member's **Reflexes** direct attribute.
        reflexes:  Reflexes,
        /// The member's **Cool** direct attribute.
        cool:      Cool,
        /// The member's **Grit** direct attribute.
        grit:      Grit,
        /// The member's **Luck** direct attribute.
        luck:      Luck,
        /// The member's **armor KEY** ([`ArmorName`]).
        armor:     ArmorName,
        /// The member's **weapon KEY** ([`WeaponName`]).
        weapon:    WeaponName,
    }
}

impl EditableMember {
    crate::support_item! {
        /// Build the **default** member the "Add member" button appends (AC4): a placeholder name
        /// and every attribute / key at its [`Default`] (the eight attributes default to `0.0`,
        /// the weapon / armor keys to the empty string). The rich edit of these fields is GTW-425.
        #[must_use]
        fn default_member() -> Self {
            Self {
                name:      GangerName::new(DEFAULT_MEMBER_NAME.to_owned()),
                speed:     Speed::default(),
                aim:       Aim::default(),
                strength:  Strength::default(),
                toughness: Toughness::default(),
                reflexes:  Reflexes::default(),
                cool:      Cool::default(),
                grit:      Grit::default(),
                luck:      Luck::default(),
                armor:     ArmorName::new(String::new()),
                weapon:    WeaponName::new(String::new()),
            }
        }
    }

    /// Build an editable member from a loaded sim [`GangMember`] (the load-from-registry path).
    pub(super) fn from_sim(member: &GangMember) -> Self {
        Self {
            name:      member.name.clone(),
            speed:     member.speed,
            aim:       member.aim,
            strength:  member.strength,
            toughness: member.toughness,
            reflexes:  member.reflexes,
            cool:      member.cool,
            grit:      member.grit,
            luck:      member.luck,
            armor:     member.armor.clone(),
            weapon:    member.weapon.clone(),
        }
    }

    crate::support_item! {
        /// Project this editable member back into the sim's serializable [`GangMember`] record —
        /// the EXACT inverse of [`from_sim`](EditableMember::from_sim) (GTW-429 C1). Every edited
        /// field maps to its sim counterpart (name + the eight attributes + weapon + armor keys),
        /// so the written `*.gang.ron` carries the full edited member and round-trips through the
        /// GTW-415 loader (C2). Declared through [`crate::support_item!`] so the round-trip test
        /// can name it.
        #[must_use]
        fn to_sim(&self) -> GangMember {
            GangMember {
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
                // GTW-505: the gang editor does not edit melee weapons yet (GTW-507+), so a
                // written member authors none — it resolves to the `fists` default in-game.
                melee_weapon: None,
            }
        }
    }

    crate::support_item! {
        /// The member's display name — the text the list-shell row shows.
        #[must_use]
        const fn name(&self) -> &GangerName {
            &self.name
        }
    }

    crate::support_item! {
        /// The member's current **weapon KEY** ([`WeaponName`]) — the text the collapsed row's
        /// weapon node shows (GTW-425 C1).
        #[must_use]
        const fn weapon(&self) -> &WeaponName {
            &self.weapon
        }
    }

    crate::support_item! {
        /// The member's current **armor KEY** ([`ArmorName`]) — the text the collapsed row's
        /// armor node shows (GTW-425 C1).
        #[must_use]
        const fn armor(&self) -> &ArmorName {
            &self.armor
        }
    }

    /// Set the member's display name — the inline name field's commit path (GTW-425 C3).
    pub(super) fn set_name(&mut self, name: GangerName) {
        self.name = name;
    }

    /// Set the member's weapon KEY — the weapon dropdown's commit path (GTW-425 C2).
    pub(super) fn set_weapon(&mut self, weapon: WeaponName) {
        self.weapon = weapon;
    }

    /// Set the member's armor KEY — the armor dropdown's commit path (GTW-425 C2).
    pub(super) fn set_armor(&mut self, armor: ArmorName) {
        self.armor = armor;
    }

    crate::support_item! {
        /// The current magnitude of one of the member's eight editable
        /// [`BaseAttribute`]s, as a bare `f32` — the value the expanded panel's matching numeric
        /// field is SEEDED with (GTW-428 C2). Each attribute newtype `Deref`s to its private `f32`.
        #[must_use]
        fn attribute(&self, attribute: BaseAttribute) -> f32 {
            match attribute {
                BaseAttribute::Speed => *self.speed,
                BaseAttribute::Aim => *self.aim,
                BaseAttribute::Strength => *self.strength,
                BaseAttribute::Toughness => *self.toughness,
                BaseAttribute::Reflexes => *self.reflexes,
                BaseAttribute::Cool => *self.cool,
                BaseAttribute::Grit => *self.grit,
                BaseAttribute::Luck => *self.luck,
            }
        }
    }

    /// Set one of the member's eight editable [`BaseAttribute`]s from a numeric-field commit's
    /// `f32` value (GTW-428 C3). Wraps the value in the matching attribute newtype.
    pub(super) const fn set_attribute(&mut self, attribute: BaseAttribute, value: f32) {
        match attribute {
            BaseAttribute::Speed => self.speed = Speed::new(value),
            BaseAttribute::Aim => self.aim = Aim::new(value),
            BaseAttribute::Strength => self.strength = Strength::new(value),
            BaseAttribute::Toughness => self.toughness = Toughness::new(value),
            BaseAttribute::Reflexes => self.reflexes = Reflexes::new(value),
            BaseAttribute::Cool => self.cool = Cool::new(value),
            BaseAttribute::Grit => self.grit = Grit::new(value),
            BaseAttribute::Luck => self.luck = Luck::new(value),
        }
    }

    crate::support_item! {
        /// The member's eight attributes grouped as the sim's [`GangerAttributes`] input record —
        /// the EXACT shape the GTW-384 [`derive_stats`](gdtf_battle_sim::derive_stats) pipeline
        /// consumes (GTW-428 C3). The editor feeds this straight to the real pipeline rather than
        /// reimplementing the derivation.
        #[must_use]
        const fn attributes(&self) -> GangerAttributes {
            GangerAttributes {
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
}

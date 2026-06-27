//! The in-app gang-editor's EDITABLE gang model (GTW-420).
//!
//! [`EditableGang`] is a NEW editor-side, in-memory editable model — distinct from the
//! immutable [`GangRoster`](gdtf_battle_sim::GangRoster) asset the `Load` flow builds. It
//! is the working copy the editor screen reads and mutates: a gang [`GangName`] plus a list
//! of [`EditableMember`]s. It is inserted as a [`Resource`] `OnEnter(DebugEditor)` (seeded
//! from a loaded gang via the [`GangRegistry`](gdtf_battle_sim::GangRegistry), or empty when
//! none is present) and removed `OnExit(DebugEditor)`, per the project's
//! state-scoped-resource convention (`bevy-traps.md` #1) — so every system reading it guards
//! with `run_if(resource_exists::<EditableGang>)` / `Option<Res<…>>`.
//!
//! Member shape: a name plus the eight direct attributes and the weapon / armor keys. The
//! GTW-420 scaffold seeded each as a minimal DEFAULT record so the list shell could show one row
//! per member; GTW-425 adds the per-member inline editing on top — the name field, weapon / armor
//! dropdowns, and delete all mutate THIS model (the EXPANDED per-member stat table is GTW-428).
//!
//! Both types are declared through [`crate::support_item!`] (and their inherent methods too),
//! so they are `pub` under the `test-support` feature — the headless tests name them through
//! [`crate::test_support`](crate::test_support) — and `pub(crate)` in the binary build, keeping
//! it `unreachable_pub`-clean (the [`LoadedSituation`](crate::states::LoadedSituation)
//! precedent).

use bevy::prelude::*;
use gdtf_battle_sim::{
    Aim, ArmorName, Cool, GangMember, GangName, GangRegistry, GangerName, Grit, Reflexes, Speed,
    Strength, Toughness, WeaponName, ganger::Luck,
};

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
    fn from_sim(member: &GangMember) -> Self {
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
    fn set_name(&mut self, name: GangerName) {
        self.name = name;
    }

    /// Set the member's weapon KEY — the weapon dropdown's commit path (GTW-425 C2).
    fn set_weapon(&mut self, weapon: WeaponName) {
        self.weapon = weapon;
    }

    /// Set the member's armor KEY — the armor dropdown's commit path (GTW-425 C2).
    fn set_armor(&mut self, armor: ArmorName) {
        self.armor = armor;
    }
}

crate::support_item! {
    /// The in-app gang editor's **editable gang model** — the working copy the editor screen
    /// edits (GTW-420).
    ///
    /// A [`Resource`] holding the gang's [`GangName`] plus its list of [`EditableMember`]s. It is
    /// the editor-side EDITABLE model, distinct from the immutable
    /// [`GangRoster`](gdtf_battle_sim::GangRoster) asset: the gang-name text field mutates
    /// [`name`](EditableGang::name) (AC3), and "Add member" appends a default
    /// [`EditableMember`] (AC4).
    ///
    /// Private inner fields with named accessors / mutators (no-bare-types rule 5): the name is a
    /// [`GangName`], the members a `Vec<EditableMember>`, both reached only through the methods
    /// below so the model is the single place they are touched.
    #[derive(Resource, Clone, PartialEq, Debug, Default)]
    struct EditableGang {
        /// The gang's editable NAME — what the gang-name text field commits to (AC3).
        name:    GangName,
        /// The gang's editable member list — what "Add member" appends to (AC4).
        members: Vec<EditableMember>,
    }
}

impl EditableGang {
    /// Build the editable model the editor opens with: the FIRST gang in `registry` (by sorted
    /// [`GangName`] for a stable pick) loaded into an editable copy, or an EMPTY model when the
    /// registry holds no gangs (AC2). [`HashMap`](bevy::platform::collections::HashMap)
    /// iteration order is unspecified, so the keys are sorted before picking, making the opened
    /// gang deterministic.
    #[must_use]
    pub(in crate::states::running::editor) fn from_registry(registry: &GangRegistry) -> Self {
        let mut names: Vec<&GangName> = registry.keys().collect();
        names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        let Some(name) = names.into_iter().next() else {
            // No gangs loaded — start empty (AC2).
            return Self::default();
        };
        let members = registry
            .roster(name)
            .map(|roster| {
                roster
                    .members
                    .iter()
                    .map(EditableMember::from_sim)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            name: name.clone(),
            members,
        }
    }

    crate::support_item! {
        /// The gang's current editable name.
        #[must_use]
        const fn name(&self) -> &GangName {
            &self.name
        }
    }

    /// Set the gang's name — the gang-name text field's commit path (AC3).
    pub(in crate::states::running::editor) fn set_name(&mut self, name: GangName) {
        self.name = name;
    }

    crate::support_item! {
        /// The gang's current members.
        #[must_use]
        fn members(&self) -> &[EditableMember] {
            &self.members
        }
    }

    crate::support_item! {
        /// Append a default member (AC4) and return its index in the list, so the caller can
        /// spawn a corresponding row keyed to it.
        fn add_default_member(&mut self) -> usize {
            self.members.push(EditableMember::default_member());
            self.members.len() - 1
        }
    }

    crate::support_item! {
        /// The member at `index`, if it exists — the row keys its controls by this index so a
        /// commit / selection can read back the just-edited member (GTW-425). [`None`] for an
        /// out-of-range index (degraded, never panics).
        #[must_use]
        fn member_at(&self, index: usize) -> Option<&EditableMember> {
            self.members.get(index)
        }
    }

    /// Set the name of the member at `index` — the inline name field's commit path
    /// (GTW-425 C3). A no-op for an out-of-range index (degraded, never panics).
    pub(in crate::states::running::editor) fn set_member_name(
        &mut self,
        index: usize,
        name: GangerName,
    ) {
        if let Some(member) = self.members.get_mut(index) {
            member.set_name(name);
        }
    }

    /// Set the weapon KEY of the member at `index` — the weapon dropdown's commit path
    /// (GTW-425 C2). A no-op for an out-of-range index (degraded, never panics).
    pub(in crate::states::running::editor) fn set_member_weapon(
        &mut self,
        index: usize,
        weapon: WeaponName,
    ) {
        if let Some(member) = self.members.get_mut(index) {
            member.set_weapon(weapon);
        }
    }

    /// Set the armor KEY of the member at `index` — the armor dropdown's commit path
    /// (GTW-425 C2). A no-op for an out-of-range index (degraded, never panics).
    pub(in crate::states::running::editor) fn set_member_armor(
        &mut self,
        index: usize,
        armor: ArmorName,
    ) {
        if let Some(member) = self.members.get_mut(index) {
            member.set_armor(armor);
        }
    }

    crate::support_item! {
        /// Remove the member at `index` from the model — the delete button's model effect
        /// (GTW-425 C4) — returning `true` when a member was actually removed (so the caller
        /// despawns exactly that row). A no-op returning `false` for an out-of-range index.
        ///
        /// NOTE: removing a member SHIFTS the indices of every later member down by one. The
        /// delete system
        /// ([`delete_member_on_press`](super::systems::delete_member_on_press)) re-keys the
        /// surviving rows' carried index after the removal so they stay in step with the model —
        /// the rows are MUTATED in place, never rebuilt (C5).
        fn remove_member(&mut self, index: usize) -> bool {
            if index < self.members.len() {
                self.members.remove(index);
                true
            } else {
                false
            }
        }
    }
}

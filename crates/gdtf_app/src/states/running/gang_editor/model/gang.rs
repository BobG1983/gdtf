//! The editable GANG resource of the gang editor's working model: the registry seed,
//! the roster projection, and the member-list mutators. Split out of the monolithic
//! `model.rs` (GTW-583); the model rationale lives on the parent `model` module.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorName,
    ganger::{GangName, GangRegistry, GangRoster, GangerName},
    weapon::WeaponName,
};

use super::member::EditableMember;
use crate::states::running::gang_editor::components::BaseAttribute;

crate::support_item! {
    /// The in-app gang editor's **editable gang model** — the working copy the editor screen
    /// edits (GTW-420).
    ///
    /// A [`Resource`] holding the gang's [`GangName`] plus its list of [`EditableMember`]s. It is
    /// the editor-side EDITABLE model, distinct from the immutable
    /// [`GangRoster`](gdtf_battle_sim::ganger::GangRoster) asset: the gang-name text field mutates
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
    pub(in crate::states::running::gang_editor) fn from_registry(registry: &GangRegistry) -> Self {
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
        /// Project the editable model into the sim's serializable `(`[`GangName`]`,
        /// `[`GangRoster`]`)` pair — the EXACT def the GTW-415 loader reads (GTW-429 C1).
        ///
        /// The gang's [`name`](EditableGang::name) becomes the registry KEY (the file stem, NOT a
        /// field of the roster — the GTW-415 key model), and each [`EditableMember`] projects to a
        /// sim [`GangMember`](gdtf_battle_sim::ganger::GangMember) via [`to_sim`](EditableMember::to_sim). The save path serializes the
        /// returned [`GangRoster`] to `assets/content/gangs/<gang_name>.gang.ron`, and the round-trip
        /// test reloads it through the same loader and asserts structural equality (C2). Declared
        /// through [`crate::support_item!`] so the round-trip test can name it.
        #[must_use]
        fn to_roster(&self) -> (GangName, GangRoster) {
            let roster = GangRoster::new(self.members.iter().map(EditableMember::to_sim));
            (self.name.clone(), roster)
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
    pub(in crate::states::running::gang_editor) fn set_name(&mut self, name: GangName) {
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
    pub(in crate::states::running::gang_editor) fn set_member_name(
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
    pub(in crate::states::running::gang_editor) fn set_member_weapon(
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
    pub(in crate::states::running::gang_editor) fn set_member_armor(
        &mut self,
        index: usize,
        armor: ArmorName,
    ) {
        if let Some(member) = self.members.get_mut(index) {
            member.set_armor(armor);
        }
    }

    /// Set one of the member-at-`index`'s editable base attributes — the expanded panel's
    /// numeric-field commit path (GTW-428 C3). A no-op for an out-of-range index (degraded, never
    /// panics).
    pub(in crate::states::running::gang_editor) fn set_member_attribute(
        &mut self,
        index: usize,
        attribute: BaseAttribute,
        value: f32,
    ) {
        if let Some(member) = self.members.get_mut(index) {
            member.set_attribute(attribute, value);
        }
    }

    crate::support_item! {
        /// Remove the member at `index` from the model — the delete button's model effect
        /// (GTW-425 C4) — returning `true` when a member was actually removed (so the caller
        /// despawns exactly that row). A no-op returning `false` for an out-of-range index.
        ///
        /// NOTE: removing a member SHIFTS the indices of every later member down by one. The
        /// delete system
        /// ([`delete_member_on_press`](super::super::systems::delete_member_on_press)) re-keys the
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

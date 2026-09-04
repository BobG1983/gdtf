//! What each editor wait condition is read from, sampled once per frame.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_assets::ContentChecksComplete;

use crate::mcp::{
    facts::EditorFactsParam,
    forms::EditorRegistries,
    wire::{ContentFamilyNet, EditorPhaseNet, EditorWaitConditionNet},
};

/// Whether a wait's condition holds right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mcp) struct ConditionMet(bool);

impl ConditionMet {
    /// Wrap whether the condition holds.
    const fn new(met: bool) -> Self {
        Self(met)
    }
}

/// How many frames one watched registry has been seen changing on.
#[derive(Deref, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(in crate::mcp) struct RegistryChangeCount(u64);

impl RegistryChangeCount {
    const fn count_one(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// One tally per watched family, so a change between the park and the check is not lost.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(in crate::mcp) struct RegistryChangeCounts([RegistryChangeCount; ContentFamilyNet::ALL.len()]);

impl RegistryChangeCounts {
    /// The tally standing against one family.
    #[must_use]
    pub(in crate::mcp) const fn count_of(&self, family: ContentFamilyNet) -> RegistryChangeCount {
        self.0[slot(family)]
    }

    /// Record that the named family's registry was written this frame.
    pub(super) const fn count_one(&mut self, family: ContentFamilyNet) {
        self.0[slot(family)].count_one();
    }
}

// Where one family's tally sits. Every family owns its own slot, which the counts test pins.
const fn slot(family: ContentFamilyNet) -> usize {
    match family {
        ContentFamilyNet::Weapon => 0,
        ContentFamilyNet::MeleeWeapon => 1,
        ContentFamilyNet::Armor => 2,
        ContentFamilyNet::Gang => 3,
        ContentFamilyNet::Terrain => 4,
        ContentFamilyNet::Theme => 5,
        ContentFamilyNet::Injury => 6,
        ContentFamilyNet::Sprite => 7,
        ContentFamilyNet::Attachment => 8,
        ContentFamilyNet::Field => 9,
    }
}

// Whether the registry this family names was written this frame.
fn changed(registries: &EditorRegistries<'_>, family: ContentFamilyNet) -> bool {
    match family {
        ContentFamilyNet::Weapon => registries
            .weapons
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::MeleeWeapon => registries
            .melee_weapon
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Armor => registries
            .armor
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Gang => registries
            .gangs
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Terrain => registries
            .terrain
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Theme => registries
            .themes
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Injury => registries
            .injuries
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Sprite => registries
            .sprites
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Attachment => registries
            .attachments
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
        ContentFamilyNet::Field => registries
            .fields
            .as_ref()
            .is_some_and(DetectChanges::is_changed),
    }
}

/// Tally every frame a watched registry changes on, so a parked wait sees one it would miss.
pub(in crate::mcp) fn count_registry_changes(
    registries: EditorRegistries,
    mut counts: ResMut<RegistryChangeCounts>,
) {
    for family in ContentFamilyNet::ALL {
        if changed(&registries, family) {
            counts.count_one(family);
        }
    }
}

/// Every source an editor wait condition resolves against.
#[derive(SystemParam)]
pub(in crate::mcp) struct EditorWaitProbe<'w> {
    facts:  EditorFactsParam<'w>,
    checks: Option<Res<'w, ContentChecksComplete>>,
    counts: Res<'w, RegistryChangeCounts>,
}

impl EditorWaitProbe<'_> {
    /// Where the editor is in its lifecycle.
    pub(in crate::mcp) fn phase(&self) -> EditorPhaseNet {
        self.facts.sample().phase()
    }

    /// The per-family tallies as they stand this frame.
    pub(in crate::mcp) fn registry_changes(&self) -> RegistryChangeCounts {
        *self.counts
    }

    /// Whether `condition` holds, measured against the tallies the call parked with.
    pub(in crate::mcp) fn met(
        &self,
        condition: EditorWaitConditionNet,
        parked_with: RegistryChangeCounts,
    ) -> ConditionMet {
        match condition {
            EditorWaitConditionNet::ChecksComplete => ConditionMet::new(self.checks.is_some()),
            EditorWaitConditionNet::RegistryRearmed { family } => {
                ConditionMet::new(*self.counts.count_of(family) > *parked_with.count_of(family))
            }
        }
    }
}

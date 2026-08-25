//! The content families an author picks from, and the members each one holds.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::key::EditorKeyNet;

/// Which registry a families read names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorFamilyNet {
    /// Terrain defs, keyed by UUID.
    Terrain,
    /// Theme defs, keyed by UUID.
    Theme,
    /// Gang rosters.
    Gang,
    /// Armor specs.
    Armor,
    /// Injury defs.
    Injury,
    /// Sprite defs.
    Sprite,
    /// Attachment specs.
    Attachment,
    /// Ranged weapon specs.
    Weapon,
    /// Melee weapon specs.
    MeleeWeapon,
}

impl EditorFamilyNet {
    /// Every family a read without a filter answers, in reply order.
    pub(in crate::net_qa) const ALL: [Self; 9] = [
        Self::Terrain,
        Self::Theme,
        Self::Gang,
        Self::Armor,
        Self::Injury,
        Self::Sprite,
        Self::Attachment,
        Self::Weapon,
        Self::MeleeWeapon,
    ];
}

/// The text the editor's own picker shows beside a key.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorFamilyLabelNet(String);

impl EditorFamilyLabelNet {
    /// Wrap a label.
    #[must_use]
    pub(in crate::net_qa) const fn new(label: String) -> Self {
        Self(label)
    }
}

/// One registry member: the key a load takes, and the label a picker shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorFamilyEntryNet {
    /// The registry key.
    key:   EditorKeyNet,
    /// What the editor shows for it.
    label: EditorFamilyLabelNet,
}

impl EditorFamilyEntryNet {
    /// Build a member entry.
    #[must_use]
    pub(in crate::net_qa) const fn new(key: EditorKeyNet, label: EditorFamilyLabelNet) -> Self {
        Self { key, label }
    }

    /// The key this entry sorts under.
    #[must_use]
    pub(in crate::net_qa) const fn key(&self) -> &EditorKeyNet {
        &self.key
    }
}

/// One family and every member it holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorFamilyRowNet {
    /// The family named.
    family:  EditorFamilyNet,
    /// Its members, sorted by rendered key.
    entries: Vec<EditorFamilyEntryNet>,
}

impl EditorFamilyRowNet {
    /// Build a family row.
    #[must_use]
    pub(in crate::net_qa) const fn new(
        family: EditorFamilyNet,
        entries: Vec<EditorFamilyEntryNet>,
    ) -> Self {
        Self { family, entries }
    }
}

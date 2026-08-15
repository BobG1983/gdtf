//! Why a save wrote no file, on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::cell::EditorCellLevelNet;
use crate::save_record::EditorSaveFault;

/// The message a serialize or write failure carried up from its source.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SaveFaultDetailNet(String);

impl SaveFaultDetailNet {
    /// Wrap the message a failing save reported.
    #[must_use]
    pub(in crate::net_qa) const fn new(detail: String) -> Self {
        Self(detail)
    }
}

/// Mirror of the editor's own save fault, one variant for one variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorSaveFaultNet {
    /// The name sanitized down to nothing, so there is no file stem.
    EmptyName,
    /// An emplacement terrain names no mounted weapon.
    MissingMountedWeapon,
    /// The theme selects no terrain to draw from.
    NoTerrain,
    /// The theme's default floor is not one of its own terrain.
    DefaultFloorNotInTerrain,
    /// A painted cell fails the editor's placement rules.
    IllegalCell(EditorCellLevelNet),
    /// The workspace root search found no marker, so there is nowhere to write.
    NoWorkspaceRoot,
    /// serde/ron could not encode the value.
    Serialize(SaveFaultDetailNet),
    /// The filesystem write or directory create failed.
    Write(SaveFaultDetailNet),
}

impl EditorSaveFaultNet {
    /// Mirror the editor's own fault, with no wildcard arm.
    #[must_use]
    pub(in crate::net_qa) fn from_fault(fault: &EditorSaveFault) -> Self {
        match fault {
            EditorSaveFault::EmptyName => Self::EmptyName,
            EditorSaveFault::MissingMountedWeapon => Self::MissingMountedWeapon,
            EditorSaveFault::NoTerrain => Self::NoTerrain,
            EditorSaveFault::DefaultFloorNotInTerrain => Self::DefaultFloorNotInTerrain,
            EditorSaveFault::IllegalCell(slot) => {
                Self::IllegalCell(EditorCellLevelNet::from_slot(*slot))
            }
            EditorSaveFault::NoWorkspaceRoot => Self::NoWorkspaceRoot,
            EditorSaveFault::Serialize(detail) => {
                Self::Serialize(SaveFaultDetailNet::new((**detail).clone()))
            }
            EditorSaveFault::Write(detail) => {
                Self::Write(SaveFaultDetailNet::new((**detail).clone()))
            }
        }
    }
}

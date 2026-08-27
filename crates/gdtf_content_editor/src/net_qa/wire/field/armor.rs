//! The Armor form's own fields, one arm per control its pieces grid draws.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::net_qa::wire::armor::{
    ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
    BodyPartNet,
};

/// One field of the Armor draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum ArmorFieldNet {
    /// The draft's name.
    Name(EditorDraftNameNet),
    /// One armor piece's damage floor.
    Floor {
        /// Which piece.
        part:  BodyPartNet,
        /// The floor it is set to.
        value: ArmorFloorNet,
    },
    /// One armor piece's protection.
    Protection {
        /// Which piece.
        part:  BodyPartNet,
        /// The protection it is set to.
        value: ArmorProtectionNet,
    },
    /// One armor piece's hardness.
    Hardness {
        /// Which piece.
        part:  BodyPartNet,
        /// The hardness it is set to.
        value: ArmorHardnessNet,
    },
    /// One armor piece's integrity.
    Integrity {
        /// Which piece.
        part:  BodyPartNet,
        /// The integrity it is set to.
        value: ArmorIntegrityNet,
    },
    /// One armor piece's material.
    Type {
        /// Which piece.
        part:  BodyPartNet,
        /// The material it is set to.
        value: ArmorTypeNet,
    },
}

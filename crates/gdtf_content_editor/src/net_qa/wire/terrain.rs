//! The Terrain form's own field values on the wire, beside its kind pick.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{LeavesBehind, LosBlocking, TerrainTag, TerrainView},
        piece::TerrainGraphicKey,
    },
    weapon::WeaponName,
};
use serde::{Deserialize, Serialize};

use super::{
    facing::{TerrainCornerNet, TerrainFacingNet},
    key::TerrainKeyNet,
    sprite::SpriteKeyNet,
};
use crate::terrain_form::FootfallChoice;

/// The hit points the one HP input writes to both the cover and the slab field.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct TerrainHpNet(u32);

impl TerrainHpNet {
    /// Wrap an HP value a client sent or the draft holds.
    pub(in crate::net_qa) const fn new(hp: u32) -> Self {
        Self(hp)
    }

    /// Read a client's HP back as the sim's own cover value.
    pub(in crate::net_qa) const fn to_cover_hp(self) -> CoverHp {
        CoverHp::new(self.0)
    }

    /// Read a client's HP back as the sim's own slab value.
    pub(in crate::net_qa) const fn to_slab_hp(self) -> SlabHp {
        SlabHp::new(self.0)
    }
}

/// How tall a piece stands, for the kinds that carry a band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum HeightBandNet {
    /// Ankle to knee.
    Low,
    /// Knee to chest.
    Mid,
    /// Chest and above.
    High,
}

impl HeightBandNet {
    /// Mirror the sim's own height band.
    pub(in crate::net_qa) const fn from_band(band: HeightBand) -> Self {
        match band {
            HeightBand::Low => Self::Low,
            HeightBand::Mid => Self::Mid,
            HeightBand::High => Self::High,
        }
    }

    /// Read a client's height band back as the sim's own.
    pub(in crate::net_qa) const fn to_band(self) -> HeightBand {
        match self {
            Self::Low => HeightBand::Low,
            Self::Mid => HeightBand::Mid,
            Self::High => HeightBand::High,
        }
    }
}

/// The footfall sound a slab plays, mirroring the form's own pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum FootfallNet {
    /// No footfall sound.
    None,
    /// Metal footfall.
    Metal,
    /// Grate footfall.
    Grate,
}

impl FootfallNet {
    /// Mirror the form's own footfall pick.
    pub(in crate::net_qa) const fn from_choice(choice: FootfallChoice) -> Self {
        match choice {
            FootfallChoice::None => Self::None,
            FootfallChoice::Metal => Self::Metal,
            FootfallChoice::Grate => Self::Grate,
        }
    }

    /// Read a client's footfall back as the form's own.
    pub(in crate::net_qa) const fn to_choice(self) -> FootfallChoice {
        match self {
            Self::None => FootfallChoice::None,
            Self::Metal => FootfallChoice::Metal,
            Self::Grate => FootfallChoice::Grate,
        }
    }
}

/// The registry key an emplacement's mounted weapon names.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct MountedWeaponNet(String);

impl MountedWeaponNet {
    /// Mirror the sim's own weapon name.
    pub(in crate::net_qa) fn from_name(name: &WeaponName) -> Self {
        Self(name.as_str().to_owned())
    }

    /// Read a client's weapon name back as the sim's own.
    pub(in crate::net_qa) fn to_name(&self) -> WeaponName {
        WeaponName::new(self.0.clone())
    }
}

/// Whether the authored override says a piece blocks pathing.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct BlocksPathingNet(bool);

impl BlocksPathingNet {
    /// Wrap the override a client sent or the draft holds.
    pub(in crate::net_qa) const fn new(blocks: bool) -> Self {
        Self(blocks)
    }
}

/// How much of a piece blocks line of sight, when the override is authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum LosBlockingNet {
    /// Full vision block.
    Full,
    /// Block up to the piece height band.
    UpToHeightBand,
    /// No vision block.
    None,
}

impl LosBlockingNet {
    /// Mirror the sim's own blocking mode.
    pub(in crate::net_qa) const fn from_blocking(blocking: LosBlocking) -> Self {
        match blocking {
            LosBlocking::Full => Self::Full,
            LosBlocking::UpToHeightBand => Self::UpToHeightBand,
            LosBlocking::None => Self::None,
        }
    }

    /// Read a client's blocking mode back as the sim's own.
    pub(in crate::net_qa) const fn to_blocking(self) -> LosBlocking {
        match self {
            Self::Full => LosBlocking::Full,
            Self::UpToHeightBand => LosBlocking::UpToHeightBand,
            Self::None => LosBlocking::None,
        }
    }
}

/// What a destroyed piece leaves standing in its cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum LeavesBehindNet {
    /// Nothing stands here afterwards.
    Nothing,
    /// Another authored def, named by its registry key.
    Piece(TerrainKeyNet),
    /// A sprite with no mechanics, named by its registry key.
    Sprite(SpriteKeyNet),
}

impl LeavesBehindNet {
    /// Mirror the sim's own choice.
    pub(in crate::net_qa) fn from_leaves_behind(leaves: &LeavesBehind) -> Self {
        match leaves {
            LeavesBehind::Nothing => Self::Nothing,
            LeavesBehind::Piece(key) => Self::Piece(TerrainKeyNet::new((**key).to_string())),
            LeavesBehind::Sprite(graphic) => Self::Sprite(SpriteKeyNet::new((**graphic).clone())),
        }
    }
}

/// One view a terrain piece can be drawn in, mirroring the sim's own with no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum TerrainViewNet {
    /// A wall's straight run along one side.
    Edge(TerrainFacingNet),
    /// A wall's turn at one corner.
    Corner(TerrainCornerNet),
    /// A cover or emplacement piece seen from one side.
    Facing(TerrainFacingNet),
    /// A door standing shut, on one side.
    Shut(TerrainFacingNet),
    /// A door standing open, on one side.
    Open(TerrainFacingNet),
    /// A staircase seen from the storey below, on one side.
    FromBelow(TerrainFacingNet),
    /// A staircase seen from the storey above, on one side.
    FromAbove(TerrainFacingNet),
    /// The one view a piece that owes a single drawing carries.
    Single,
}

impl TerrainViewNet {
    /// Mirror the sim's own view.
    pub(in crate::net_qa) const fn from_view(view: TerrainView) -> Self {
        match view {
            TerrainView::Edge(facing) => Self::Edge(TerrainFacingNet::from_facing(facing)),
            TerrainView::Corner(corner) => Self::Corner(TerrainCornerNet::from_corner(corner)),
            TerrainView::Facing(facing) => Self::Facing(TerrainFacingNet::from_facing(facing)),
            TerrainView::Shut(facing) => Self::Shut(TerrainFacingNet::from_facing(facing)),
            TerrainView::Open(facing) => Self::Open(TerrainFacingNet::from_facing(facing)),
            TerrainView::FromBelow(facing) => {
                Self::FromBelow(TerrainFacingNet::from_facing(facing))
            }
            TerrainView::FromAbove(facing) => {
                Self::FromAbove(TerrainFacingNet::from_facing(facing))
            }
            TerrainView::Single => Self::Single,
        }
    }

    /// Read a client's view back as the sim's own.
    pub(in crate::net_qa) const fn to_view(self) -> TerrainView {
        match self {
            Self::Edge(facing) => TerrainView::Edge(facing.to_facing()),
            Self::Corner(corner) => TerrainView::Corner(corner.to_corner()),
            Self::Facing(facing) => TerrainView::Facing(facing.to_facing()),
            Self::Shut(facing) => TerrainView::Shut(facing.to_facing()),
            Self::Open(facing) => TerrainView::Open(facing.to_facing()),
            Self::FromBelow(facing) => TerrainView::FromBelow(facing.to_facing()),
            Self::FromAbove(facing) => TerrainView::FromAbove(facing.to_facing()),
            Self::Single => TerrainView::Single,
        }
    }
}

/// The sprite-def key one view names, by file stem.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct TerrainViewSpriteNet(String);

impl TerrainViewSpriteNet {
    /// Mirror the sim's own graphic key.
    pub(in crate::net_qa) fn from_key(key: &TerrainGraphicKey) -> Self {
        Self((**key).clone())
    }

    /// Read a client's key back as the sim's own.
    pub(in crate::net_qa) fn to_key(&self) -> TerrainGraphicKey {
        TerrainGraphicKey::new(self.0.clone())
    }
}

/// One tag of the Terrain form's tick-box row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum TerrainTagNet {
    /// Can be opened and closed.
    Openable,
    /// A staircase, for the view derivation.
    Stair,
    /// Blocks vision.
    BlocksVision,
    /// Blocks pathfinding.
    BlocksPathfinding,
    /// Cannot be destroyed.
    Indestructible,
}

impl TerrainTagNet {
    /// Every tag the form's row draws, for a case that walks them all.
    #[cfg(test)]
    pub(in crate::net_qa) const ALL: [Self; 5] = [
        Self::Openable,
        Self::Stair,
        Self::BlocksVision,
        Self::BlocksPathfinding,
        Self::Indestructible,
    ];

    /// Mirror the sim's own tag.
    pub(in crate::net_qa) const fn from_tag(tag: TerrainTag) -> Self {
        match tag {
            TerrainTag::Openable => Self::Openable,
            TerrainTag::Stair => Self::Stair,
            TerrainTag::BlocksVision => Self::BlocksVision,
            TerrainTag::BlocksPathfinding => Self::BlocksPathfinding,
            TerrainTag::Indestructible => Self::Indestructible,
        }
    }

    /// Read a client's tag back as the sim's own.
    pub(in crate::net_qa) const fn to_tag(self) -> TerrainTag {
        match self {
            Self::Openable => TerrainTag::Openable,
            Self::Stair => TerrainTag::Stair,
            Self::BlocksVision => TerrainTag::BlocksVision,
            Self::BlocksPathfinding => TerrainTag::BlocksPathfinding,
            Self::Indestructible => TerrainTag::Indestructible,
        }
    }
}

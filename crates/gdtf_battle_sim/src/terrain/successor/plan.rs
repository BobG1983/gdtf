//! Reading a destroyed piece's def to decide what stands in its cell next.

use crate::{
    effects::on_death::OnDeathEffect,
    metric::CellLevel,
    occupancy_sync::TerrainPieceDestroyed,
    situation::terrain_resolve::{
        ResolvedCoverPiece, ResolvedSlabPiece, resolve_cover_def, resolve_slab_def,
    },
    terrain::{
        def::{LeavesBehind, TerrainDefRegistry, TerrainUuid},
        entity::{TerrainIndexKey, TerrainPieceKind},
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
};

/// The def key and placement facing read off the destroyed piece's own entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PieceIdentity {
    pub(super) piece:  TerrainUuid,
    pub(super) facing: TerrainFacing,
}

/// A successor piece, resolved against its own def and turned the destroyed piece's way.
pub(super) struct SuccessorCover {
    pub(super) piece:    TerrainUuid,
    pub(super) facing:   TerrainFacing,
    pub(super) resolved: ResolvedCoverPiece,
}

/// A successor slab, resolved against its own def and turned the destroyed piece's way.
pub(super) struct SuccessorSlab {
    pub(super) piece:    TerrainUuid,
    pub(super) facing:   TerrainFacing,
    pub(super) resolved: ResolvedSlabPiece,
}

/// What stands in the cell once the destroyed piece is gone.
pub(super) enum LeftBehind {
    /// Nothing: the cell is cleared for the kind the message named.
    Nothing,
    /// A sprite with no mechanics at all.
    Sprite(TerrainGraphicKey),
    /// A wall, cover or emplacement piece.
    Cover(Box<SuccessorCover>),
    /// A floor slab.
    Slab(Box<SuccessorSlab>),
}

/// One destroyed piece, and what replaces it.
pub(super) struct Plan {
    pub(super) key:      TerrainIndexKey,
    pub(super) at:       CellLevel,
    pub(super) kind:     TerrainPieceKind,
    pub(super) left:     LeftBehind,
    pub(super) on_death: Vec<OnDeathEffect>,
}

/// The index key a destroyed-piece message names, split by the kind it carries.
pub(super) const fn index_key(message: &TerrainPieceDestroyed) -> TerrainIndexKey {
    match message.kind {
        TerrainPieceKind::Slab => TerrainIndexKey::Slab(message.at),
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => {
            TerrainIndexKey::Cover(message.at)
        }
    }
}

/// Decide what a destroyed piece leaves behind, reading its def and the successor's.
pub(super) fn plan_for(
    message: &TerrainPieceDestroyed,
    identity: Option<PieceIdentity>,
    defs: Option<&TerrainDefRegistry>,
) -> Plan {
    let cleared = Plan {
        key:      index_key(message),
        at:       message.at,
        kind:     message.kind,
        left:     LeftBehind::Nothing,
        on_death: Vec::new(),
    };
    let Some(identity) = identity else {
        return cleared;
    };
    let Some(def) = defs.and_then(|registry| registry.def(&identity.piece)) else {
        return cleared;
    };
    match &def.leaves_behind {
        LeavesBehind::Nothing => cleared,
        LeavesBehind::Sprite(graphic) => Plan {
            left: LeftBehind::Sprite(graphic.clone()),
            ..cleared
        },
        LeavesBehind::Piece(key) => successor_plan(*key, identity.facing, defs, cleared),
    }
}

// The successor def resolved for the destroyed piece's kind, or the cleared plan.
fn successor_plan(
    key: TerrainUuid,
    facing: TerrainFacing,
    defs: Option<&TerrainDefRegistry>,
    cleared: Plan,
) -> Plan {
    let Some(def) = defs.and_then(|registry| registry.def(&key)) else {
        return cleared;
    };
    match cleared.kind {
        TerrainPieceKind::Slab => match resolve_slab_def(&key, def) {
            Some(resolved) => Plan {
                left: LeftBehind::Slab(Box::new(SuccessorSlab {
                    piece: key,
                    facing,
                    resolved,
                })),
                on_death: def.on_death.clone(),
                ..cleared
            },
            None => cleared,
        },
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => {
            match resolve_cover_def(&key, def) {
                Some(resolved) => Plan {
                    left: LeftBehind::Cover(Box::new(SuccessorCover {
                        piece: key,
                        facing,
                        resolved,
                    })),
                    on_death: def.on_death.clone(),
                    ..cleared
                },
                None => cleared,
            }
        }
    }
}

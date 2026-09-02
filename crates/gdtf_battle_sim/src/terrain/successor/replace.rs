//! Replace every destroyed piece with what its def's `leaves_behind` names.

use bevy::{
    platform::collections::HashSet,
    prelude::{Commands, MessageReader, MessageWriter, Query},
};

use super::{
    components::{ReplacedPiece, SlabLeftOpen},
    plan::{PieceIdentity, Plan, index_key, plan_for},
    spawn::apply,
    writes::SuccessorWrites,
};
use crate::{
    occupancy_sync::TerrainPieceDestroyed,
    terrain::{
        def::TerrainUuid,
        entity::{BlocksPathfinding, BlocksVision, TerrainCell, TerrainIndexKey},
        facing::TerrainFacing,
    },
};

type PieceRow<'a> = (
    &'a TerrainCell,
    &'a TerrainUuid,
    &'a TerrainFacing,
    Option<&'a ReplacedPiece>,
);

/// Put what each destroyed piece's def leaves behind into its cell, and mark the old piece.
pub fn replace_destroyed_piece(
    mut destroyed: MessageReader<TerrainPieceDestroyed>,
    pieces: Query<PieceRow<'static>>,
    mut commands: Commands,
    mut writes: SuccessorWrites,
    mut opened: MessageWriter<SlabLeftOpen>,
) {
    let mut acted: HashSet<TerrainIndexKey> = HashSet::default();
    let mut plans: Vec<Plan> = Vec::new();
    for message in destroyed.read() {
        let key = index_key(message);
        if !acted.insert(key) {
            continue;
        }
        let standing = writes.index.as_deref().and_then(|index| index.get(&key));
        let row = standing.and_then(|entity| pieces.get(entity).ok());
        if row.is_some_and(|(_, _, _, replaced)| replaced.is_some()) {
            continue;
        }
        if let Some(entity) = standing {
            commands
                .entity(entity)
                .remove::<BlocksPathfinding>()
                .remove::<BlocksVision>()
                .insert(ReplacedPiece);
        }
        let identity = row.map(|(_, piece, facing, _)| PieceIdentity {
            piece:  *piece,
            facing: *facing,
        });
        plans.push(plan_for(message, identity, writes.defs.as_deref()));
    }
    for plan in plans {
        apply(&mut commands, &mut writes, &mut opened, plan);
    }
}

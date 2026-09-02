use super::{super::TerrainPieceDestroyed, support::*};
use crate::{occupancy::TerrainKind, terrain::entity::TerrainPieceKind};

#[test]
fn cover_destroyed_message_clears_the_cell() {
    let mut app = headless_app();
    let smashed = key(30, 31, 3);
    let intact = key(0, 0, 0);
    set_cell_terrain(&mut app, smashed, TerrainKind::Cover);
    set_cell_terrain(&mut app, intact, TerrainKind::Cover);

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(smashed, TerrainPieceKind::Cover));
    app.update();

    assert_eq!(
        cell_terrain(&app, smashed),
        Some(TerrainKind::Open),
        "a destroyed Cover piece that leaves nothing behind clears its own cell on the \
         occupancy grid",
    );
    assert_eq!(
        cell_terrain(&app, intact),
        Some(TerrainKind::Cover),
        "an unrelated cell keeps the piece standing in it",
    );
}

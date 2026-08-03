use super::{
    assembler::PlacedPrefab,
    deploy::{DeploymentZone, DeploymentZones},
    engine::{ProcgenCursor, StagedProcgenRegistries},
    error::PackingError,
    fill::FilledPlacement,
    findings::{EmittedLevel, ProcgenFinding},
    geometry::RegionRect,
    tuning::ProcgenTuning,
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid, UuidThemeRegistry},
    metric::{Cell, CellLevel, Level},
    rng::ProcgenRng,
    situation::{CoverSpawn, FloorSpawn, Situation, SlabSpawn},
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainPieceKind,
    },
};

/// Run the full procgen pipeline to an [`EmittedLevel`].
///
/// # Errors
///
/// Returns [`PackingError`] from any stage (assemble, fill, or later packing failure).
pub fn generate_level(
    prefabs: &PrefabRegistry,
    themes: &UuidThemeRegistry,
    terrain_defs: &TerrainDefRegistry,
    theme: ThemeUuid,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    tuning: &ProcgenTuning,
) -> Result<EmittedLevel, PackingError> {
    let registries = StagedProcgenRegistries {
        prefabs,
        themes,
        terrain_defs,
        tuning,
    };
    let mut cursor = ProcgenCursor::new(theme, grid_size);
    loop {
        cursor.step(registries, rng)?;
        if let Some(emitted) = cursor.emitted() {
            return Ok(emitted.clone());
        }
    }
}

#[must_use]
pub fn emit_level(
    filled: &FilledPlacement,
    theme: ThemeUuid,
    grid_size: GridSize,
    themes: &UuidThemeRegistry,
    terrain_defs: &TerrainDefRegistry,
) -> EmittedLevel {
    let placement = filled.placement();
    let mut findings: Vec<ProcgenFinding> = Vec::new();

    let default_floor = themes.default_floor(&theme).unwrap_or_else(|| {
        findings.push(ProcgenFinding::MissingThemeDefaultFloor { theme });
        crate::terrain::def::TerrainUuid::default()
    });

    let mut situation = Situation::new();
    situation.theme = theme;
    situation.grid_size = grid_size;
    situation.default_floor = default_floor;

    pour_prefab(
        placement.player(),
        &mut situation,
        terrain_defs,
        &mut findings,
    );
    pour_prefab(
        placement.enemy(),
        &mut situation,
        terrain_defs,
        &mut findings,
    );
    for placed in filled.fill() {
        pour_prefab(placed, &mut situation, terrain_defs, &mut findings);
    }

    for rect in filled.dead_space() {
        floor_region(*rect, default_floor, &mut situation);
    }

    let zones = DeploymentZones::new(
        DeploymentZone::new(placement.player().anchor(), placement.player().region()),
        DeploymentZone::new(placement.enemy().anchor(), placement.enemy().region()),
    );

    EmittedLevel {
        situation,
        findings,
        zones,
    }
}

fn pour_prefab(
    placed: &PlacedPrefab,
    situation: &mut Situation,
    terrain_defs: &TerrainDefRegistry,
    findings: &mut Vec<ProcgenFinding>,
) {
    let origin = placed.region().origin();
    let spec = placed.prefab().spec();

    for placement in &spec.placements {
        let at = translate(placement.at, origin);
        match classify(placement.piece, terrain_defs) {
            PlacedKind::Slab => situation.slabs.push(SlabSpawn::new(at, placement.piece)),
            PlacedKind::Cover => situation.walls.push(CoverSpawn::new(at, placement.piece)),
            PlacedKind::Unresolved => {
                situation.walls.push(CoverSpawn::new(at, placement.piece));
                let finding = ProcgenFinding::UnresolvedTerrainPiece {
                    piece: placement.piece,
                };
                if !findings.contains(&finding) {
                    findings.push(finding);
                }
            }
        }
    }
}

enum PlacedKind {
    Cover,
    Slab,
    Unresolved,
}

fn classify(piece: TerrainUuid, terrain_defs: &TerrainDefRegistry) -> PlacedKind {
    match terrain_defs.def(&piece).map(|def| def.sim_kind.kind()) {
        Some(TerrainPieceKind::Slab) => PlacedKind::Slab,
        Some(TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement) => {
            PlacedKind::Cover
        }
        None => PlacedKind::Unresolved,
    }
}

fn floor_region(rect: RegionRect, default_floor: TerrainUuid, situation: &mut Situation) {
    let origin = rect.origin();
    let footprint = rect.footprint();
    for dy in 0..footprint.height() {
        for dx in 0..footprint.width() {
            let cell = Cell::new(origin.x + dx, origin.y + dy);
            situation.floors.push(FloorSpawn::new(
                CellLevel::new(cell, Level::new(0)),
                default_floor,
            ));
        }
    }
}

fn translate(local: CellLevel, origin: Cell) -> CellLevel {
    let cell = Cell::new(local.x + origin.x, local.y + origin.y);
    CellLevel::new(cell, local.level())
}

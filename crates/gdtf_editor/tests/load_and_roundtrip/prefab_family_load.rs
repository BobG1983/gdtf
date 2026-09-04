//! The editor reads `content/maps/` through `PrefabsFamily`, and an opened prefab converts back.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

use bevy::{asset::uuid::Uuid, prelude::*};
use gdtf_assets::ContentFolderHandle;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, PrefabRegistry, PrefabSpec, SpawnRole,
        TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        facing::TerrainFacing,
    },
};
use gdtf_content_families::PrefabsFamily;
use gdtf_editor::{
    CurrentEditLevel, EditorMap, EditorState, MapEditorSession, editor_map_to_prefab, open_prefab,
    prefab_save_path_in, serialize_prefab,
};
use gdtf_test_utils::advance_until;

use crate::support::editor_app_with_asset_root;

const THEME: ThemeUuid = ThemeUuid::new(Uuid::from_u128(0x0132_2300_0001));

const SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2300_0002));

const FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2300_0003));

// The file stem the family keys the loaded prefab by.
const STEM: &str = "family_entry";

// Painted off the facing default, so a load that drops the field cannot pass.
const AUTHORED_FACING: TerrainFacing = TerrainFacing::East;

fn authored_size() -> GridSize {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1))
        .unwrap_or_else(|_| GridSize::default())
}

// A size the session starts on that is not the authored one, so the size assertion can fail.
fn other_size() -> GridSize {
    GridSize::new(GridWidth::new(8), GridHeight::new(8), GridLevels::new(2))
        .unwrap_or_else(|_| GridSize::default())
}

// Three slabs on the ground storey of an empty map, every one legal under the placement rules.
fn authored_spec() -> PrefabSpec {
    let cells = [Cell::new(0, 0), Cell::new(1, 2), Cell::new(2, 1)];
    PrefabSpec::new(
        THEME,
        authored_size(),
        SpawnRole::Fill,
        cells
            .into_iter()
            .map(|cell| {
                TerrainPlacementEntry::new(
                    SLAB,
                    CellLevel::new(cell, Level::new(0)),
                    AUTHORED_FACING,
                )
            })
            .collect(),
    )
}

fn slab_def() -> TerrainDef {
    TerrainDef {
        key:            SLAB,
        display_name:   TerrainDisplayName::new("Family Slab".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(80),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        views:          TerrainViews::new(Vec::new()),
        tags:           Vec::new(),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

fn themes() -> UuidThemeRegistry {
    UuidThemeRegistry::new([(
        THEME,
        UuidThemeDef {
            key:           THEME,
            display_name:  ThemeDisplayName::new("Family Hive".to_owned()),
            default_floor: FLOOR,
            terrain:       vec![SLAB],
        },
    )])
}

// The order `editor_map_to_prefab` sorts its own output into.
fn sorted(mut placements: Vec<TerrainPlacementEntry>) -> Vec<TerrainPlacementEntry> {
    placements.sort_by(|a, b| {
        (a.at.x, a.at.y, a.at.z, *a.piece).cmp(&(b.at.x, b.at.y, b.at.z, *b.piece))
    });
    placements
}

// Write one authored prefab under `root`, through the editor's own save-path helper.
fn author_one_prefab(root: &std::path::Path, spec: &PrefabSpec) -> Result<(), std::io::Error> {
    let path = prefab_save_path_in(root, "Family Hive", spec.size, STEM);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let Ok(text) = serialize_prefab(spec) else {
        unreachable!("an authored prefab spec serializes to RON");
    };
    std::fs::write(path, text)
}

#[test]
fn the_editor_loads_the_prefab_folder_and_an_opened_prefab_converts_back() {
    let Ok(dir) = tempfile::tempdir() else {
        unreachable!("creating the TempDir assets root must succeed");
    };
    let spec = authored_spec();
    assert!(
        author_one_prefab(dir.path(), &spec).is_ok(),
        "the case writes its own prefab under the temp root before the editor reads it",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|state| *state.get() == EditorState::Editing)
    });
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<PrefabsFamily>>()
            .is_some(),
        "the editor registers PrefabsFamily, so its folder handle is in the world by the time \
         Editing starts. Without that registration there is no prefab load at all, and waiting \
         on the registry below would spin instead of failing",
    );

    advance_until(&mut app, |app| {
        app.world().get_resource::<PrefabRegistry>().is_some()
    });
    let Some(registry) = app.world().get_resource::<PrefabRegistry>() else {
        unreachable!("the wait above returns only once the registry is in the world");
    };
    assert!(
        !registry.is_empty(),
        "one authored prefab sits under the temp root, so the family resolves it into the \
         registry rather than falling back to an empty one",
    );

    let Some(loaded) = registry.iter().find(|prefab| ***prefab.name() == *STEM) else {
        unreachable!("the family keys the loaded prefab by its file stem `{STEM}`");
    };
    let loaded = loaded.spec().clone();

    let terrain = TerrainDefRegistry::new([(SLAB, slab_def())]);
    let mut session = MapEditorSession::new(THEME, Some(FLOOR), other_size());
    let mut map = EditorMap::new();
    let mut edit_level = CurrentEditLevel::ground();
    assert_ne!(
        session.grid_size(),
        loaded.size,
        "the session starts on a size that is not the prefab's, so the size assertion below \
         has somewhere to fail",
    );

    open_prefab(&mut session, &mut map, &mut edit_level, &themes(), &loaded);

    let Ok(built) = editor_map_to_prefab(&map, &terrain, &session) else {
        unreachable!("every authored cell is a legal slab placement, so the conversion runs");
    };
    assert_eq!(
        built.size, loaded.size,
        "the conversion reads the session's grid extent, which the open wrote from the spec",
    );
    assert_eq!(
        built.placements,
        sorted(loaded.placements.clone()),
        "the conversion answers the cells the open painted, sorted the way it sorts its own \
         output, so a dropped placement shows up here",
    );
}

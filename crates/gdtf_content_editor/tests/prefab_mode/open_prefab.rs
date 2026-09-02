use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, PrefabSpec, SpawnRole, TerrainPlacementEntry,
        ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};
use gdtf_content_editor::{CurrentEditLevel, EditorMap, MapEditorSession, open_prefab};

const SESSION_THEME: ThemeUuid = ThemeUuid::new(Uuid::from_u128(0x0132_2200_0001));

const SPEC_THEME: ThemeUuid = ThemeUuid::new(Uuid::from_u128(0x0132_2200_0002));

const SESSION_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2200_0011));

const SPEC_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2200_0012));

const PIECE: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2200_0021));

// Painted off the facing default, so a load that drops the field cannot pass.
const AUTHORED_FACING: TerrainFacing = TerrainFacing::West;

fn size(levels: u8) -> GridSize {
    GridSize::new(
        GridWidth::new(4),
        GridHeight::new(4),
        GridLevels::new(levels),
    )
    .unwrap_or_else(|_| GridSize::default())
}

fn theme_def(key: ThemeUuid, floor: TerrainUuid, name: &str) -> UuidThemeDef {
    UuidThemeDef {
        key,
        display_name: ThemeDisplayName::new(name.to_owned()),
        default_floor: floor,
        terrain: vec![PIECE],
    }
}

// Two themes with different default floors, so a floor read off the wrong one is visible.
fn themes() -> UuidThemeRegistry {
    UuidThemeRegistry::new([
        (
            SESSION_THEME,
            theme_def(SESSION_THEME, SESSION_FLOOR, "Session Hive"),
        ),
        (SPEC_THEME, theme_def(SPEC_THEME, SPEC_FLOOR, "Spec Hive")),
    ])
}

fn session_on_the_other_theme(levels: u8) -> MapEditorSession {
    MapEditorSession::new(SESSION_THEME, Some(SESSION_FLOOR), size(levels))
}

fn flat_spec() -> PrefabSpec {
    PrefabSpec::new(
        SPEC_THEME,
        size(1),
        SpawnRole::Fill,
        vec![TerrainPlacementEntry::new(
            PIECE,
            CellLevel::new(Cell::new(1, 1), Level::new(0)),
            AUTHORED_FACING,
        )],
    )
}

// A two-storey spec whose upper entry the session's own one-storey extent would reject.
fn two_storey_spec() -> PrefabSpec {
    PrefabSpec::new(
        SPEC_THEME,
        size(2),
        SpawnRole::Fill,
        vec![
            TerrainPlacementEntry::new(
                PIECE,
                CellLevel::new(Cell::new(0, 0), Level::new(0)),
                TerrainFacing::North,
            ),
            TerrainPlacementEntry::new(
                PIECE,
                CellLevel::new(Cell::new(2, 3), Level::new(1)),
                AUTHORED_FACING,
            ),
        ],
    )
}

#[test]
fn the_loaded_theme_carries_that_themes_own_default_floor() {
    let themes = themes();
    let spec = flat_spec();
    let mut session = session_on_the_other_theme(1);
    let mut map = EditorMap::new();
    let mut edit_level = CurrentEditLevel::ground();

    open_prefab(&mut session, &mut map, &mut edit_level, &themes, &spec);

    assert_eq!(
        session.theme(),
        spec.theme,
        "the open selects the theme the prefab was authored under, so the palette and the \
         preview resolve against it",
    );
    assert_eq!(
        session.default_floor(),
        Some(SPEC_FLOOR),
        "select_theme takes the theme and its floor together, so passing None here would \
         leave the session on the new theme carrying no floor, which editor.session reports \
         straight",
    );
}

#[test]
fn the_map_holds_exactly_the_authored_cells_and_nothing_else() {
    let themes = themes();
    let spec = two_storey_spec();
    let mut session = session_on_the_other_theme(1);
    let mut edit_level = CurrentEditLevel::ground();

    let stale = CellLevel::new(Cell::new(3, 3), Level::new(0));
    let mut map = EditorMap::new();
    assert!(
        map.paint_at(stale, PIECE, TerrainFacing::South, size(1)),
        "the case needs a cell painted before the open, at a slot the spec does not name",
    );

    open_prefab(&mut session, &mut map, &mut edit_level, &themes, &spec);

    assert_eq!(
        map.painted_count(),
        spec.placements.len(),
        "every authored placement is painted, and nothing else is. Bounds-checking against \
         the session's own one-storey extent would drop the entry on storey 1",
    );
    let upper = CellLevel::new(Cell::new(2, 3), Level::new(1));
    assert_eq!(
        map.tile_at_level(upper).map(|piece| piece.facing()),
        Some(AUTHORED_FACING),
        "the upper-storey entry reads back with the facing it was authored with",
    );
    assert!(
        map.tile_at_level(stale).is_none(),
        "the open replaces the map outright, so what the author had painted is gone",
    );
}

#[test]
fn a_one_storey_prefab_pulls_the_edit_level_back_to_the_ground() {
    let themes = themes();
    let spec = flat_spec();
    let mut session = session_on_the_other_theme(3);
    let mut map = EditorMap::new();
    let mut edit_level = CurrentEditLevel::jumped(Level::new(2), size(3));
    assert_eq!(
        edit_level.level(),
        Level::new(2),
        "the case starts on the top storey of a three-storey session, or the clamp has \
         nowhere to move it from",
    );

    open_prefab(&mut session, &mut map, &mut edit_level, &themes, &spec);

    assert_eq!(
        edit_level.level(),
        Level::new(0),
        "a one-storey prefab has no storey 2, so the open clamps the edit storey into the \
         extent it just wrote",
    );
}

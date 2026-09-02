use bevy::asset::uuid::Uuid;
use bevy_egui::egui;
use gdtf_battle_presenter::{IsolateView, ViewMode};
use gdtf_battle_sim::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef,
        UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};

use crate::{
    canvas::CurrentEditLevel,
    editor_map::EditorMap,
    egui_shell::prefab::{
        controls_ui::{
            EditedPrefab, PREFAB_PICKER_SALT, StoreyToggles, TerrainLibrary, controls_panel,
        },
        level_rail::RailUiState,
    },
    open::prefab_candidates,
    save_record::LastSaveRecord,
    session::MapEditorSession,
};

const THEME: ThemeUuid = ThemeUuid::new(Uuid::from_u128(0x0132_2000_0001));

const FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2000_0002));

const PIECE: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0132_2000_0003));

// The one authored prefab the picker lists, named apart from every other painted string.
const PREFAB_NAME: &str = "entry room";

// The line the picker draws in place of the combo, ending in one U+2026.
const LOADING: &str = "(loading…)";

// The pane the test hands egui, big enough that nothing is clipped away.
fn screen() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))
}

// The id the test gives the Ui it builds, which the combo's button id is derived from.
fn ui_id() -> egui::Id {
    egui::Id::new("prefab_picker_draw")
}

// The id `ComboBox::from_id_salt` lands on: `ui.id()` hashed with the salt as an `IdSalt`.
fn button_id() -> egui::Id {
    ui_id().with(egui::IdSalt::new(PREFAB_PICKER_SALT))
}

fn listed_size() -> GridSize {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(2))
        .unwrap_or_else(|_| GridSize::default())
}

// Two cells on two storeys, so a load that paints nothing and one that paints one both fail.
fn listed_spec() -> PrefabSpec {
    PrefabSpec::new(
        THEME,
        listed_size(),
        SpawnRole::Fill,
        vec![
            TerrainPlacementEntry::new(
                PIECE,
                CellLevel::new(Cell::new(0, 0), Level::new(0)),
                TerrainFacing::North,
            ),
            TerrainPlacementEntry::new(
                PIECE,
                CellLevel::new(Cell::new(1, 2), Level::new(1)),
                TerrainFacing::East,
            ),
        ],
    )
}

fn prefabs() -> PrefabRegistry {
    let mut registry = PrefabRegistry::default();
    registry.insert(Prefab::new(
        PrefabName::new(PREFAB_NAME.to_owned()),
        listed_spec(),
    ));
    registry
}

fn themes() -> UuidThemeRegistry {
    UuidThemeRegistry::new([(
        THEME,
        UuidThemeDef {
            key:           THEME,
            display_name:  ThemeDisplayName::new("Picker Hive".to_owned()),
            default_floor: FLOOR,
            terrain:       vec![PIECE],
        },
    )])
}

// The label the picker draws for the one listed prefab, read off the same rows it draws.
fn listed_label(registry: &PrefabRegistry) -> String {
    match prefab_candidates(registry).first() {
        Some((_, label)) => label.clone(),
        None => unreachable!("a one-prefab registry answers one picker row"),
    }
}

// One string egui painted, with the middle of the galley it painted it into.
struct DrawnText {
    text:   String,
    centre: egui::Pos2,
}

// Every string egui painted this pass, gathered out of the shapes it emitted.
fn drawn_text(shape: &egui::epaint::Shape, into: &mut Vec<DrawnText>) {
    match shape {
        egui::epaint::Shape::Vec(shapes) => {
            for child in shapes {
                drawn_text(child, into);
            }
        }
        egui::epaint::Shape::Text(text) => into.push(DrawnText {
            text:   text.galley.text().to_owned(),
            centre: text.pos + text.galley.size() / 2.0,
        }),
        _ => {}
    }
}

fn says(painted: &[DrawnText], wanted: &str) -> bool {
    painted.iter().any(|drawn| drawn.text == wanted)
}

fn painted_strings(painted: &[DrawnText]) -> Vec<&str> {
    painted.iter().map(|drawn| drawn.text.as_str()).collect()
}

// A press and a release at one point, the pair egui reads back as a click.
fn press(at: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(at),
        egui::Event::PointerButton {
            pos:       at,
            button:    egui::PointerButton::Primary,
            pressed:   true,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn release(at: egui::Pos2) -> Vec<egui::Event> {
    vec![egui::Event::PointerButton {
        pos:       at,
        button:    egui::PointerButton::Primary,
        pressed:   false,
        modifiers: egui::Modifiers::NONE,
    }]
}

// The model borrows the controls panel writes, held across the passes one case runs.
struct Canvas {
    session:    MapEditorSession,
    map:        EditorMap,
    edit_level: CurrentEditLevel,
    view:       ViewMode,
    isolate:    IsolateView,
    rail_state: RailUiState,
    save_name:  String,
    last_save:  LastSaveRecord,
}

impl Canvas {
    fn new() -> Self {
        Self {
            session:    MapEditorSession::default(),
            map:        EditorMap::new(),
            edit_level: CurrentEditLevel::ground(),
            view:       ViewMode::default(),
            isolate:    IsolateView::Off,
            rail_state: RailUiState::default(),
            save_name:  String::new(),
            last_save:  LastSaveRecord::default(),
        }
    }

    // Draw the whole controls panel once, and read back what egui painted.
    fn pass(
        &mut self,
        ctx: &egui::Context,
        prefabs: Option<&PrefabRegistry>,
        themes: Option<&UuidThemeRegistry>,
        events: Vec<egui::Event>,
    ) -> Vec<DrawnText> {
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(screen()),
            events,
            ..Default::default()
        });
        {
            let mut ui = egui::Ui::new(
                ctx.clone(),
                ui_id(),
                egui::UiBuilder::new()
                    .layer_id(egui::LayerId::background())
                    .max_rect(screen()),
            );
            controls_panel(
                &mut ui,
                EditedPrefab {
                    map: &mut self.map,
                    session: &mut self.session,
                    edit_level: &mut self.edit_level,
                    prefabs,
                },
                StoreyToggles {
                    view:    &mut self.view,
                    isolate: &mut self.isolate,
                },
                TerrainLibrary {
                    terrain: None,
                    themes,
                },
                &mut self.rail_state,
                &mut self.save_name,
                &mut self.last_save,
            );
        }
        let output = ctx.end_pass();
        let mut painted = Vec::new();
        for shape in &output.shapes {
            drawn_text(&shape.shape, &mut painted);
        }
        painted
    }
}

// A closed combo paints its button and nothing else, so the popup is opened by id first.
//
// It then takes two more passes to read the rows back: an `egui::Area` is laid out invisibly on
// the first pass it is shown in, because egui does not know its size yet.
fn open_and_draw(
    canvas: &mut Canvas,
    ctx: &egui::Context,
    prefabs: Option<&PrefabRegistry>,
    themes: Option<&UuidThemeRegistry>,
) -> Vec<DrawnText> {
    canvas.pass(ctx, prefabs, themes, Vec::new());
    egui::Popup::open_id(ctx, button_id().with("popup"));
    assert!(
        egui::ComboBox::is_open(ctx, button_id()),
        "the popup id this test opens must be the one the combo reads, or a later pass would \
         paint no rows and the failure would look like a missing picker",
    );
    canvas.pass(ctx, prefabs, themes, Vec::new());
    canvas.pass(ctx, prefabs, themes, Vec::new())
}

#[test]
fn the_open_picker_lists_every_authored_prefab_once_its_popup_is_open() {
    let (prefabs, themes) = (prefabs(), themes());
    let label = listed_label(&prefabs);
    let ctx = egui::Context::default();
    let mut canvas = Canvas::new();

    let painted = open_and_draw(&mut canvas, &ctx, Some(&prefabs), Some(&themes));

    assert!(
        says(&painted, &label),
        "the open picker draws one row per authored prefab, so `{label}` must be painted while \
         the popup is open: {:?}",
        painted_strings(&painted),
    );
}

#[test]
fn a_missing_prefab_registry_draws_the_loading_line_and_no_row() {
    let (prefabs, themes) = (prefabs(), themes());
    let label = listed_label(&prefabs);
    let ctx = egui::Context::default();
    let mut canvas = Canvas::new();

    let painted = open_and_draw(&mut canvas, &ctx, None, Some(&themes));

    assert!(
        says(&painted, LOADING),
        "the prefab folder resolves after Editing starts, so the picker says it is loading \
         rather than drawing an empty combo: {:?}",
        painted_strings(&painted),
    );
    assert!(
        !says(&painted, &label),
        "with no registry there is nothing to list, so no row may be painted: {:?}",
        painted_strings(&painted),
    );
}

#[test]
fn a_missing_theme_registry_draws_the_loading_line_and_no_row() {
    let prefabs = prefabs();
    let label = listed_label(&prefabs);
    let ctx = egui::Context::default();
    let mut canvas = Canvas::new();

    let painted = open_and_draw(&mut canvas, &ctx, Some(&prefabs), None);

    assert!(
        says(&painted, LOADING),
        "an open reads the theme's default floor off the theme registry, so with no registry \
         the picker refuses to draw at all: {:?}",
        painted_strings(&painted),
    );
    assert!(
        !says(&painted, &label),
        "no click may leave the session on a theme carrying no floor, so no row is offered: \
         {:?}",
        painted_strings(&painted),
    );
}

#[test]
fn clicking_a_row_loads_that_prefab_onto_the_canvas() {
    let (prefabs, themes) = (prefabs(), themes());
    let label = listed_label(&prefabs);
    let spec = listed_spec();
    let ctx = egui::Context::default();
    let mut canvas = Canvas::new();
    assert_ne!(
        canvas.session.grid_size(),
        spec.size,
        "the session must start on a size that is not the prefab's, or the size assertion has \
         nowhere to fail",
    );

    let painted = open_and_draw(&mut canvas, &ctx, Some(&prefabs), Some(&themes));
    let Some(row) = painted.iter().find(|drawn| drawn.text == label) else {
        unreachable!(
            "the open popup paints the row this case clicks: {:?}",
            painted_strings(&painted)
        );
    };
    let at = row.centre;

    canvas.pass(&ctx, Some(&prefabs), Some(&themes), press(at));
    canvas.pass(&ctx, Some(&prefabs), Some(&themes), release(at));

    assert_eq!(
        canvas.session.grid_size(),
        spec.size,
        "the clicked row loads that prefab, which puts its own grid extent on the session",
    );
    assert_eq!(
        canvas.session.theme(),
        spec.theme,
        "the clicked row loads that prefab's theme, so the palette resolves against it",
    );
    assert_eq!(
        canvas.map.painted_count(),
        spec.placements.len(),
        "every authored placement is painted, including the one on the upper storey",
    );
}

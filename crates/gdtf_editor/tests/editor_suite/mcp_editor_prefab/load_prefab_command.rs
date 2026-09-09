use bevy::app::App;
use cobalt_test_utils::advance_until;
use gdtf_battle_sim::level::{PrefabRegistry, PrefabSpec};
use gdtf_editor::prefab_candidates;

use crate::{
    mcp_editor_prefab::{
        names::EDITOR_LOAD_PREFAB,
        rows::{LoadPrefabOutcomeRow, LoadPrefabReplyRow, SessionGridRow},
        setup::{prefab_app_and_client, read_map},
    },
    mcp_shared::{
        outcome::ran_body,
        socket::{Client, run_editor},
        support::{TestError, TestResult},
    },
};

/// The read this case drives to see the loaded extent and theme back off the session.
const EDITOR_SESSION: &str = "editor.session";

/// The name and key one shipped prefab is filed under, plus the spec behind it.
struct Asked {
    name: String,
    spec: PrefabSpec,
}

impl Asked {
    fn arguments(&self) -> String {
        let size = self.spec.size;
        format!(
            "(name: \"{}\", theme: \"{}\", size: (width: {}, height: {}, levels: {}), role: {:?})",
            self.name,
            *self.spec.theme,
            *size.width(),
            *size.height(),
            *size.levels(),
            self.spec.role,
        )
    }
}

// Whichever prefab the picker lists first, so authoring a new one cannot redden this case.
fn a_listed_prefab(app: &App) -> Result<Asked, TestError> {
    let Some(registry) = app.world().get_resource::<PrefabRegistry>() else {
        return Err("the wait above returns only once the registry is in the world".into());
    };
    let rows = prefab_candidates(registry);
    let Some((prefab, _)) = rows.first() else {
        return Err("the shipped assets hold at least one authored prefab".into());
    };
    Ok(Asked {
        name: (**prefab.name()).clone(),
        spec: prefab.spec().clone(),
    })
}

fn session_grid(app: &mut App, client: &mut Client) -> Result<SessionGridRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SESSION, "()"))?;
    ran_body(&reply, EDITOR_SESSION)
}

#[test]
fn an_authored_prefab_loads_its_extent_theme_and_cells_onto_the_canvas() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<PrefabRegistry>()
            .is_some_and(|registry| !registry.is_empty())
    });
    let asked = a_listed_prefab(&app)?;

    let before = session_grid(&mut app, &mut client)?;
    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_LOAD_PREFAB, &asked.arguments()),
    )?;
    let body: LoadPrefabReplyRow = ran_body(&reply, EDITOR_LOAD_PREFAB)?;
    let (grid_size, placements) = match body.outcome {
        LoadPrefabOutcomeRow::Opened {
            grid_size,
            placements,
        } => (grid_size, placements),
        LoadPrefabOutcomeRow::NoSuchPrefab {
            name,
            theme,
            size,
            role,
        } => {
            return Err(format!(
                "`{}` names a prefab the registry holds, so the load opens it rather than \
                 missing on ({name}, {theme}, {size:?}, {role:?})",
                asked.name
            )
            .into());
        }
    };
    assert_eq!(
        (grid_size.width, grid_size.height, grid_size.levels),
        (
            *asked.spec.size.width(),
            *asked.spec.size.height(),
            *asked.spec.size.levels()
        ),
        "the reply carries the extent the prefab was authored at",
    );
    assert_eq!(
        placements,
        asked.spec.placements.len(),
        "the reply counts the cells the open painted",
    );

    let after = session_grid(&mut app, &mut client)?;
    assert_eq!(
        after.theme,
        (*asked.spec.theme).to_string(),
        "the load selects the prefab's own theme in the session, which was `{}` before it: \
         {after:?}",
        before.theme,
    );
    assert_eq!(
        (
            after.grid_size.width,
            after.grid_size.height,
            after.grid_size.levels
        ),
        (
            *asked.spec.size.width(),
            *asked.spec.size.height(),
            *asked.spec.size.levels()
        ),
        "the session reads back the loaded extent, not the one it started on: {before:?}",
    );

    let ground = read_map(&mut app, &mut client, 0)?;
    let mut read: Vec<(i32, i32)> = ground
        .painted
        .iter()
        .map(|row| (row.cell.x, row.cell.y))
        .collect();
    read.sort_unstable();
    let mut authored: Vec<(i32, i32)> = asked
        .spec
        .placements
        .iter()
        .filter(|entry| entry.at.z == 0)
        .map(|entry| (entry.at.x, entry.at.y))
        .collect();
    authored.sort_unstable();
    assert_eq!(
        read, authored,
        "the ground storey holds the cells the prefab authored there and nothing else: \
         {ground:?}",
    );
    Ok(())
}

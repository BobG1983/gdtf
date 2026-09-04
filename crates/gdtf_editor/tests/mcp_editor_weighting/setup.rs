use bevy::app::App;
use cobalt_mcp_protocol::message::QaResponse;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryRegistry, InjuryTables, InjuryWeighting, WeightedInjuryEntry},
    severity::Severity,
};
use gdtf_editor::{EditorMode, WeightingDraft};

use crate::{
    harness::editing_app_and_client,
    names::{
        EDITOR_LAST_SAVE, EDITOR_LIST_OP, EDITOR_SAVE_WEIGHTING, EDITOR_SELECT_WEIGHTING_TABLE,
        EDITOR_SET_FIELD, EDITOR_SET_MODE, EDITOR_WEIGHTING,
    },
    outcome::ran_body,
    rows::{
        CategoryRow, ContextRow, LastSaveReplyRow, LastSaveRow, ListOpReplyRow, SaveOutcomeRow,
        SaveWeightingReplyRow, SetFieldReplyRow, WeightingRow, WeightingTableRow,
    },
    socket::{Client, run_editor},
    support::TestError,
};

/// The mode tab the world has open right now.
pub(crate) fn editor_mode(app: &App) -> Result<EditorMode, TestError> {
    let Some(mode) = app.world().get_resource::<EditorMode>() else {
        return Err("the mode tab is a resource the editor creates on entering Editing".into());
    };
    Ok(*mode)
}

/// An editing app with the Injury tab open, which every weighting command needs.
pub(crate) fn injury_tab_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, mut client) = editing_app_and_client()?;
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Injury)"))?;
    let open = editor_mode(&app)?;
    if open != EditorMode::Injury {
        return Err(format!("the Injury tab must be open before a case runs, got {open:?}").into());
    }
    Ok((app, client))
}

/// The weighting the draft holds right now.
pub(crate) fn draft_weighting(app: &App) -> Result<InjuryWeighting, TestError> {
    let Some(draft) = app.world().get_resource::<WeightingDraft>() else {
        return Err(
            "the weighting draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.weighting().clone())
}

/// The injury tables the editor loaded before it entered Editing.
pub(crate) fn injury_tables(app: &App) -> Result<InjuryTables, TestError> {
    let Some(tables) = app.world().get_resource::<InjuryTables>() else {
        return Err("the editor reached Editing, so its injury tables are loaded".into());
    };
    Ok(tables.clone())
}

/// Every injury key the live registry holds, in the order the row combo offers them.
pub(crate) fn sorted_injury_keys(app: &App) -> Result<Vec<String>, TestError> {
    let Some(registry) = app.world().get_resource::<InjuryRegistry>() else {
        return Err("the editor reached Editing, so its injury registry is loaded".into());
    };
    let mut keys: Vec<String> = registry.iter().map(|(key, _)| key.to_string()).collect();
    keys.sort();
    Ok(keys)
}

/// One bucket of the live tables, read as the rows a reply would carry.
pub(crate) fn live_rows(
    tables: &InjuryTables,
    category: InjuryCategory,
    context: DamageContext,
    severity: Severity,
) -> Vec<WeightingRow> {
    tables
        .table_for_category(category, context, severity)
        .map(|table| {
            table
                .iter()
                .map(|entry| WeightingRow {
                    injury: entry.injury.to_string(),
                    weight: *entry.weight,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The draft's own weighting read as the reply body a client compares against.
pub(crate) fn table_of(weighting: &InjuryWeighting) -> WeightingTableRow {
    let rows = |bucket: &[WeightedInjuryEntry]| {
        bucket
            .iter()
            .map(|entry| WeightingRow {
                injury: entry.injury.to_string(),
                weight: *entry.weight,
            })
            .collect()
    };
    WeightingTableRow {
        category: category_row(weighting.category),
        context:  context_row(weighting.context),
        minor:    rows(&weighting.minor),
        major:    rows(&weighting.major),
        critical: rows(&weighting.critical),
    }
}

const fn category_row(category: InjuryCategory) -> CategoryRow {
    match category {
        InjuryCategory::Head => CategoryRow::Head,
        InjuryCategory::Torso => CategoryRow::Torso,
        InjuryCategory::Arm => CategoryRow::Arm,
        InjuryCategory::Leg => CategoryRow::Leg,
    }
}

const fn context_row(context: DamageContext) -> ContextRow {
    match context {
        DamageContext::Ranged => ContextRow::Ranged,
        DamageContext::Melee => ContextRow::Melee,
        DamageContext::Fall => ContextRow::Fall,
    }
}

/// Run `editor.select_weighting_table` and read the table the reply carries.
pub(crate) fn select_table(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<WeightingTableRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SELECT_WEIGHTING_TABLE, arguments))?;
    ran_body(&reply, EDITOR_SELECT_WEIGHTING_TABLE)
}

/// Run `editor.weighting` and read the table the reply carries.
pub(crate) fn weighting(
    app: &mut App,
    client: &mut Client,
) -> Result<WeightingTableRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_WEIGHTING, "()"))?;
    ran_body(&reply, EDITOR_WEIGHTING)
}

/// Run `editor.save_weighting` and read what the writer reported.
pub(crate) fn save_weighting(
    app: &mut App,
    client: &mut Client,
) -> Result<SaveOutcomeRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SAVE_WEIGHTING, "()"))?;
    let body: SaveWeightingReplyRow = ran_body(&reply, EDITOR_SAVE_WEIGHTING)?;
    Ok(body.outcome)
}

/// Run `editor.last_save` for one mode and read the records the reply carries.
pub(crate) fn last_save_rows(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<Vec<LastSaveRow>, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_LAST_SAVE, arguments))?;
    let body: LastSaveReplyRow = ran_body(&reply, EDITOR_LAST_SAVE)?;
    Ok(body.records)
}

/// Run `editor.set_field` and read the field the reply carries.
pub(crate) fn set_field(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<SetFieldReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SET_FIELD, arguments))?;
    ran_body(&reply, EDITOR_SET_FIELD)
}

/// Run `editor.list_op` and read the bucket the reply carries.
pub(crate) fn list_op(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<ListOpReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_LIST_OP, arguments))?;
    ran_body(&reply, EDITOR_LIST_OP)
}

/// Run one command and hand back whatever outcome it answered.
pub(crate) fn try_run(
    app: &mut App,
    client: &mut Client,
    command: &'static str,
    arguments: &str,
) -> Result<QaResponse, TestError> {
    client.exchange(app, &run_editor(command, arguments))
}

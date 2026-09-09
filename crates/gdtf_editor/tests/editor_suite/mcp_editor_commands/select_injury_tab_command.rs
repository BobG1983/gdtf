use bevy::app::App;
use gdtf_editor::{EditorMode, InjurySubTab};

use crate::{
    mcp_editor_commands::{
        names::{EDITOR_SELECT_INJURY_TAB, EDITOR_SET_MODE},
        rows::{SelectInjuryTabReplyRow, SetModeReplyRow, SubTabRow},
    },
    mcp_shared::{
        harness::editing_app_and_client,
        mirror::ModeRow,
        outcome::{ran_body, unavailable_code},
        socket::{Client, run_editor},
        support::{TestError, TestResult},
        world::editor_mode,
    },
};

// The Injury sub-tab the world has open right now.
fn injury_sub_tab(app: &App) -> Result<InjurySubTab, TestError> {
    let Some(sub_tab) = app.world().get_resource::<InjurySubTab>() else {
        return Err(
            "the Injury sub-tab is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(*sub_tab)
}

// The whole args body `editor.select_injury_tab` takes.
fn tab_args(tab: &str) -> String {
    format!("(tab: {tab})")
}

// Open a top-level tab and check the reply names it, so a case starts where it means to.
fn open_tab(app: &mut App, client: &mut Client, mode: ModeRow) -> Result<(), TestError> {
    let args = format!("(mode: {mode:?})");
    let reply = client.exchange(app, &run_editor(EDITOR_SET_MODE, &args))?;
    let body: SetModeReplyRow = ran_body(&reply, EDITOR_SET_MODE)?;
    if body.mode != mode {
        return Err(format!(
            "the reply that opened the tab must name {mode:?}, got {:?}",
            body.mode
        )
        .into());
    }
    app.update();
    Ok(())
}

/// An editing app with the Injury tab open.
fn injury_tab_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, ModeRow::Injury)?;
    let mode = editor_mode(&app)?;
    if mode != EditorMode::Injury {
        return Err(format!("the Injury tab must be open before a case runs, got {mode:?}").into());
    }
    Ok((app, client))
}

#[test]
fn selecting_a_sub_tab_moves_the_resource_and_the_reply_names_the_open_one() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    assert_eq!(
        injury_sub_tab(&app)?,
        InjurySubTab::Def,
        "a fresh authoring session starts on the def form, so the Tables case moves it",
    );

    let args = tab_args("Tables");
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_INJURY_TAB, &args))?;
    let body: SelectInjuryTabReplyRow = ran_body(&reply, EDITOR_SELECT_INJURY_TAB)?;
    app.update();
    assert_eq!(
        body.tab,
        SubTabRow::Tables,
        "the reply names the sub-tab now open, so a client needs no second read",
    );
    assert_eq!(
        injury_sub_tab(&app)?,
        InjurySubTab::Tables,
        "the world holds the sub-tab the call named, read out of the frame that answered it",
    );

    let args = tab_args("Def");
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_INJURY_TAB, &args))?;
    let body: SelectInjuryTabReplyRow = ran_body(&reply, EDITOR_SELECT_INJURY_TAB)?;
    app.update();
    assert_eq!(
        body.tab,
        SubTabRow::Def,
        "a second call moves the sub-tab back, so the choice is not one-way",
    );
    assert_eq!(
        injury_sub_tab(&app)?,
        InjurySubTab::Def,
        "the world followed the second call as well as the first",
    );
    Ok(())
}

#[test]
fn the_call_is_refused_on_another_top_level_tab_and_the_sub_tab_stands() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let args = tab_args("Tables");
    client.exchange(&mut app, &run_editor(EDITOR_SELECT_INJURY_TAB, &args))?;
    app.update();
    let before = injury_sub_tab(&app)?;

    open_tab(&mut app, &mut client, ModeRow::Armor)?;
    let args = tab_args("Def");
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_INJURY_TAB, &args))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the sub-tab row belongs to the Injury form, so the write is refused while another \
         top-level tab is open",
    );
    app.update();
    assert_eq!(
        injury_sub_tab(&app)?,
        before,
        "a refused call writes nothing: the sub-tab must hold the value it held before",
    );
    Ok(())
}

#[test]
fn leaving_the_injury_tab_and_coming_back_keeps_the_sub_tab_the_author_left_on() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let args = tab_args("Tables");
    client.exchange(&mut app, &run_editor(EDITOR_SELECT_INJURY_TAB, &args))?;
    app.update();

    open_tab(&mut app, &mut client, ModeRow::Armor)?;
    open_tab(&mut app, &mut client, ModeRow::Injury)?;

    assert_eq!(
        injury_sub_tab(&app)?,
        InjurySubTab::Tables,
        "the sub-tab is state-scoped to Editing, not to the open tab, so a round trip through \
         another top-level tab leaves the author where they were",
    );
    Ok(())
}

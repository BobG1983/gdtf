use cobalt_mcp_protocol::message::McpRequest;
use gdtf_editor::EditorMode;

use crate::{
    mcp_editor_prefab::{
        names::{EDITOR_SET_MODE, PREFAB_TAB_ARGUMENTS, PREFAB_TAB_COMMANDS},
        refusal::{catalogue_refusal_of, refusal_of},
        setup::prefab_app_and_client,
    },
    mcp_shared::{
        load_case::reply_answered_during_load, socket::run_editor, support::TestResult,
        world::editor_mode,
    },
};

#[test]
fn every_prefab_command_refuses_another_tab_with_a_note_the_load_pass_does_not_use() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Terrain)"))?;
    app.update();
    let opened = editor_mode(&app)?;
    assert_eq!(
        opened,
        EditorMode::Terrain,
        "the case must leave the Prefab tab before it asks, or every command would answer from \
         the tab it is scoped to",
    );

    let loading = reply_answered_during_load(McpRequest::Catalogue, "the catalogue request")?;
    for (command, arguments) in PREFAB_TAB_COMMANDS.into_iter().zip(PREFAB_TAB_ARGUMENTS) {
        let reply = client.exchange(&mut app, &run_editor(command, arguments))?;
        let (code, tab_note) = refusal_of(&reply)?;
        assert_eq!(
            code, "WrongState",
            "`{command}` is refused for the tab the editor has open, which is what WrongState \
             names",
        );
        assert!(
            !tab_note.is_empty(),
            "`{command}`'s refusal carries the line that tells a client which tab to open",
        );
        let (_, phase_note) = catalogue_refusal_of(&loading, command)?;
        assert_ne!(
            tab_note, phase_note,
            "`{command}` checks the phase before the tab, so a call on the wrong tab carries \
             the tab's own note and a call during Load carries the phase's. One note for both \
             leaves a client waiting for Editing that is already in it",
        );
    }
    Ok(())
}

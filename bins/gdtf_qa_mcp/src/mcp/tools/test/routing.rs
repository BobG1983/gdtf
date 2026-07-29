//! Where a call goes: the wire-name mapping and the host each tool acts on.

use crate::{
    hosts::QaHost,
    mcp::tools::{ToolName, name::ALL},
};

/// Every wire name round-trips through `from_wire`, and an unknown name resolves to
/// `None`. Walking `ALL` (rather than a hand-listed table) keeps a newly added tool in.
#[test]
fn wire_names_round_trip() {
    for tool in ALL.iter().copied() {
        assert_eq!(ToolName::from_wire(tool.wire_name()), Some(tool));
    }
    assert_eq!(ToolName::from_wire("nope"), None);
}

/// Every tool names a host, and the four editor tools name the EDITOR — a tool that
/// resolved to the game's link would carry an editor request to a process that answers it
/// `BadRequest` (GTW-808).
#[test]
fn every_tool_names_its_host() {
    for tool in ALL.iter().copied() {
        let expected = if matches!(
            tool,
            ToolName::GetEditorQueryOptions
                | ToolName::QueryEditor
                | ToolName::LaunchEditor
                | ToolName::StopEditor
        ) {
            QaHost::Editor
        } else {
            QaHost::Game
        };
        assert_eq!(tool.host(), expected, "{} names its host", tool.wire_name());
    }
    assert!(ToolName::LaunchEditor.is_launch());
    assert!(ToolName::StopEditor.is_stop());
    assert!(!ToolName::QueryEditor.is_launch());
    assert!(!ToolName::QueryEditor.is_stop());
}

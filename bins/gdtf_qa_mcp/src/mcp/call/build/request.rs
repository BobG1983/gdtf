//! [`build_request`] — the tool → [`QaRequest`] map.

use gdtf_qa_protocol::envelope::QaRequest;
use serde_json::Value;

use crate::mcp::{
    call::build::{
        command::{parse_focus_command, parse_stepper_command},
        intent::parse_intent,
        scalars::{
            parse_frame_delay, parse_max, parse_name, parse_seed, parse_situation, parse_token,
        },
    },
    tools::ToolName,
};

/// Build the [`QaRequest`] a tool call maps onto from its `arguments`.
///
/// # Errors
///
/// A human-readable message when an argument is missing or the wrong shape.
pub fn build_request(tool: ToolName, args: &Value) -> Result<QaRequest, String> {
    match tool {
        ToolName::SendInput => Ok(QaRequest::Inject(parse_intent(args)?)),
        ToolName::QueryState => Ok(QaRequest::GetBattleState),
        ToolName::GetOutput => Ok(QaRequest::GetOutput {
            max: parse_max(args)?,
        }),
        ToolName::TakeScreenshot => Ok(QaRequest::TakeScreenshot {
            name: parse_name(args)?,
        }),
        ToolName::ScreenshotAfter => Ok(QaRequest::ScreenshotAfter {
            intent:      parse_intent(args)?,
            frame_delay: parse_frame_delay(args)?,
            name:        parse_name(args)?,
        }),
        ToolName::AppFlow => Ok(QaRequest::GetAppFlow),
        ToolName::StartBattle => Ok(QaRequest::StartBattle {
            situation: parse_situation(args)?,
            seed:      parse_seed(args)?,
        }),
        ToolName::StepperControl => Ok(QaRequest::StepperControl(parse_stepper_command(args)?)),
        ToolName::ActivateMenuItem => Ok(QaRequest::ActivateMenuItem(parse_token(args)?)),
        ToolName::FocusControl => Ok(QaRequest::FocusControl(parse_focus_command(args)?)),
        // The lifecycle tools are handled before this point (in `handle_tool_call`), so
        // they never map onto a wire request; reaching here would be a routing bug.
        ToolName::LaunchGame | ToolName::StopGame => {
            Err("launch_game / stop_game are host-local tools, not game requests".to_owned())
        }
    }
}

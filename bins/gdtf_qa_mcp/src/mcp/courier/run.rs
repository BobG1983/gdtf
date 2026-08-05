use gdtf_qa_protocol::{
    command::{AwaitBudget, CaptureRider, CommandArgsRon, CommandName, CommandOutcome, RunOptions},
    ids::ShotName,
    message::RunCommand,
};
use serde_json::{Value, json};

use super::attach::attachment_blocks;
use crate::{lifecycle::WorkingDir, mcp::content::tool_error};

fn parse_command(args: &Value) -> Result<CommandName, String> {
    match args.get("command") {
        None | Some(Value::Null) => {
            Err("`run` needs a `command` argument — read the names from `commands`".to_owned())
        }
        Some(Value::String(name)) => Ok(CommandName::from_owned(name.clone())),
        Some(other) => Err(format!("`command` must be a string, not {other}")),
    }
}

fn parse_arguments(args: &Value) -> Result<CommandArgsRon, String> {
    match args.get("arguments") {
        None | Some(Value::Null) => Ok(CommandArgsRon::new("()".to_owned())),
        Some(Value::String(text)) => Ok(CommandArgsRon::new(text.clone())),
        Some(other) => Err(format!(
            "`arguments` must be a string of compact RON, not {other}"
        )),
    }
}

fn parse_await_ready(args: &Value) -> Result<Option<AwaitBudget>, String> {
    match args.get("await_ready") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map_or_else(
            || Err("`await_ready` must be a non-negative whole number of seconds".to_owned()),
            |seconds| Ok(Some(AwaitBudget::new(seconds))),
        ),
    }
}

fn parse_capture(args: &Value) -> Result<Option<CaptureRider>, String> {
    match args.get("capture") {
        None | Some(Value::Null | Value::Bool(false)) => Ok(None),
        Some(Value::Bool(true)) => Ok(Some(CaptureRider::new(None))),
        Some(Value::String(stem)) => Ok(Some(CaptureRider::new(Some(ShotName::new(stem.clone()))))),
        Some(other) => Err(format!(
            "`capture` must be true or a file-stem string, not {other}"
        )),
    }
}

pub(in crate::mcp) fn parse_run(args: &Value) -> Result<RunCommand, String> {
    Ok(RunCommand::with_options(
        parse_command(args)?,
        parse_arguments(args)?,
        RunOptions::new(parse_await_ready(args)?, parse_capture(args)?),
    ))
}

pub(in crate::mcp) fn render_outcome(
    outcome: &CommandOutcome,
    child_dir: Option<&WorkingDir>,
) -> Value {
    match outcome {
        CommandOutcome::Ran { reply, attachments } => {
            let mut content = vec![json!({
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "outcome": "Ran",
                    "reply": reply.as_str(),
                }))
                .unwrap_or_else(|_| "<unserializable>".to_owned()),
            })];
            content.extend(attachment_blocks(attachments, child_dir));
            json!({ "content": content, "isError": false })
        }
        CommandOutcome::Unavailable { code, note } => tool_error(&describe(&json!({
            "outcome": "Unavailable",
            "code": code,
            "note": note.as_str(),
        }))),
        CommandOutcome::BadArguments { detail, schema } => tool_error(&describe(&json!({
            "outcome": "BadArguments",
            "detail": detail.as_str(),
            "schema": schema.as_str(),
        }))),
        CommandOutcome::Unknown { known } => tool_error(&describe(&json!({
            "outcome": "Unknown",
            "known": known.iter().map(CommandName::as_str).collect::<Vec<&str>>(),
        }))),
    }
}

fn describe(body: &Value) -> String {
    serde_json::to_string_pretty(body).unwrap_or_else(|_| "<unserializable>".to_owned())
}

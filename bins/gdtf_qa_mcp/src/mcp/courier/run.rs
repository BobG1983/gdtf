//! The `run` tool — build one `Run` from a call's arguments, and render its outcome
//! (GTW-942).

use gdtf_qa_protocol::{
    command::{
        AwaitBudget, CaptureRider, CommandArgsJson, CommandName, CommandOutcome, RunOptions,
    },
    envelope::RunCommand,
    ids::ShotName,
};
use serde_json::{Value, json};

use super::attach::attachment_blocks;
use crate::{lifecycle::WorkingDir, mcp::content::tool_error};

/// Read the required `command` name.
///
/// The name is passed through as the caller wrote it: which commands exist is the HOST's to
/// know, and it answers an unknown one with `Unknown` carrying every name it does offer —
/// one round trip and the typo self-corrects. A list kept here too would be a second copy of
/// that truth, free to go stale the moment a host gains a command.
fn parse_command(args: &Value) -> Result<CommandName, String> {
    match args.get("command") {
        None | Some(Value::Null) => {
            Err("`run` needs a `command` argument — read the names from `commands`".to_owned())
        }
        Some(Value::String(name)) => Ok(CommandName::from_owned(name.clone())),
        Some(other) => Err(format!("`command` must be a string, not {other}")),
    }
}

/// Read the optional `arguments` object, defaulting to the empty object.
///
/// The body is OPAQUE here — only the command's own `Args` type gives it meaning, and only
/// the host that owns that command decodes it. The courier's only job is to check it is an
/// object and hand the text across; a body the command will not accept comes back as
/// `BadArguments` carrying that command's own schema.
fn parse_arguments(args: &Value) -> Result<CommandArgsJson, String> {
    let body = match args.get("arguments") {
        None | Some(Value::Null) => return Ok(CommandArgsJson::new("{}".to_owned())),
        Some(value @ Value::Object(_)) => value,
        Some(other) => return Err(format!("`arguments` must be an object, not {other}")),
    };
    serde_json::to_string(body).map_or_else(
        |err| {
            Err(format!(
                "`arguments` could not be re-encoded as JSON: {err}"
            ))
        },
        |text| Ok(CommandArgsJson::new(text)),
    )
}

/// Read the optional `await_ready` budget, in whole seconds.
fn parse_await_ready(args: &Value) -> Result<Option<AwaitBudget>, String> {
    match args.get("await_ready") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map_or_else(
            || Err("`await_ready` must be a non-negative whole number of seconds".to_owned()),
            |seconds| Ok(Some(AwaitBudget::new(seconds))),
        ),
    }
}

/// Read the optional `capture` rider — `true` for a host-chosen file stem, or a string to
/// pick one.
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

/// Build the `Run` body a `run` call maps onto.
///
/// # Errors
///
/// A human-readable message when an argument is missing or the wrong shape.
pub(in crate::mcp) fn parse_run(args: &Value) -> Result<RunCommand, String> {
    Ok(RunCommand::with_options(
        parse_command(args)?,
        parse_arguments(args)?,
        RunOptions::new(parse_await_ready(args)?, parse_capture(args)?),
    ))
}

/// A command's reply body as a JSON VALUE, or as its raw text if it will not parse.
///
/// The body travels as text produced by the command's own `Serialize`, so it parses in
/// practice; handing the text back verbatim rather than dropping it is what makes the
/// exception visible instead of silent.
fn reply_value(text: &str) -> Value {
    serde_json::from_str::<Value>(text).unwrap_or_else(|_| Value::String(text.to_owned()))
}

/// Turn a `Run`'s outcome into the MCP content the caller reads.
///
/// A command that RAN is a success: its reply is the text block, and anything it attached
/// follows as its own block. The other three outcomes are tool errors (`isError: true`),
/// because in each of them the call did NOT do what was asked — and each carries, as JSON,
/// exactly what the caller needs to fix it in one round trip: the precondition that is
/// missing, the schema the arguments failed against, or every name this host does offer.
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
                    "reply": reply_value(reply.as_str()),
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
            "schema": super::commands::schema_document(schema.as_str()),
        }))),
        CommandOutcome::Unknown { known } => tool_error(&describe(&json!({
            "outcome": "Unknown",
            "known": known.iter().map(CommandName::as_str).collect::<Vec<&str>>(),
        }))),
    }
}

/// A refusal body as pretty JSON text — the payload the three error outcomes carry.
fn describe(body: &Value) -> String {
    serde_json::to_string_pretty(body).unwrap_or_else(|_| "<unserializable>".to_owned())
}

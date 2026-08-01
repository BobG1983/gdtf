//! The `commands` tool — read a host's live catalogue and render it (GTW-942).

use gdtf_qa_protocol::command::{CommandCatalogue, CommandEntry, CommandName};
use serde_json::{Value, json};

use crate::mcp::content::{text_content, tool_error};

/// How much of a catalogue row a call asked for.
///
/// A typed alternative to threading a bare string: the two levels exist because a full
/// catalogue is mostly JSON Schema text, and a client browsing "what can I call here" wants
/// the names, not several kilobytes of schema per row it has not chosen yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mcp) enum CatalogueDetail {
    /// Names, summaries, timings and availability — no schemas.
    Summary,
    /// The same rows with each command's two derived schemas attached.
    Full,
}

impl CatalogueDetail {
    /// The word a call spells this detail level with, and a schema advertises.
    const fn label(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Full => "Full",
        }
    }

    /// Every detail level, in the order the tool schema lists them.
    pub(in crate::mcp) const ALL: [Self; 2] = [Self::Summary, Self::Full];

    /// Resolve a caller's word, or `None` for a word that names no level.
    fn from_label(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.label() == word)
    }

    /// Every level's word — what the tool schema enumerates, so the schema can never
    /// advertise a spelling [`from_label`](Self::from_label) rejects.
    pub(in crate::mcp) fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(Self::label).collect()
    }
}

/// Read the optional `detail` argument, defaulting to [`Summary`](CatalogueDetail::Summary).
///
/// # Errors
///
/// A message listing every legal level when the argument is present but names none — never
/// a silent fall back to `Summary`, which would answer a question the caller did not ask.
pub(in crate::mcp) fn parse_detail(args: &Value) -> Result<CatalogueDetail, String> {
    match args.get("detail") {
        None | Some(Value::Null) => Ok(CatalogueDetail::Summary),
        Some(Value::String(word)) => CatalogueDetail::from_label(word).ok_or_else(|| {
            format!(
                "unknown `detail` {word:?}; expected one of: {}",
                CatalogueDetail::labels().join(", ")
            )
        }),
        Some(other) => Err(format!("`detail` must be a string, not {other}")),
    }
}

/// Read the optional `command` filter — one command's row instead of the whole list.
///
/// # Errors
///
/// A message when the argument is present but is not a string.
pub(in crate::mcp) fn parse_filter(args: &Value) -> Result<Option<CommandName>, String> {
    match args.get("command") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => Ok(Some(CommandName::from_owned(name.clone()))),
        Some(other) => Err(format!("`command` must be a string, not {other}")),
    }
}

/// A derived schema document as a JSON VALUE, or as its raw text if it will not parse.
///
/// The schemas travel as text — a JSON document inside a RON frame — and a client wants
/// real JSON, not an escaped string it has to parse itself. A document that does not parse
/// is handed back verbatim rather than dropped: that is a defect in the host worth seeing,
/// and the host's own `assert_schemas_parse` is what stops it reaching here.
///
/// Shared with [`run`](super::run): a `BadArguments` outcome carries the SAME argument
/// schema a `Full` catalogue row publishes, so the two must present it the same way.
pub(super) fn schema_document(text: &str) -> Value {
    serde_json::from_str::<Value>(text).unwrap_or_else(|_| Value::String(text.to_owned()))
}

/// Render one catalogue row at the requested detail.
///
/// `schemas` is `null` at [`Summary`](CatalogueDetail::Summary) and an object carrying the
/// row's two derived documents at [`Full`](CatalogueDetail::Full) — ONE field either way, so
/// a client reads the same path in both cases and can tell "not asked for" from "empty".
fn render_entry(entry: &CommandEntry, detail: CatalogueDetail) -> Value {
    let schemas = match detail {
        CatalogueDetail::Summary => Value::Null,
        CatalogueDetail::Full => json!({
            "arguments": schema_document(entry.arguments.as_str()),
            "reply": schema_document(entry.reply.as_str()),
        }),
    };
    json!({
        "command": entry.command.as_str(),
        "summary": entry.summary.as_str(),
        "timing": entry.timing,
        "availability": entry.availability,
        "schemas": schemas,
    })
}

/// Render a host's catalogue, filtered to one command when the call named one.
///
/// A `command` filter that matches nothing is a TOOL ERROR listing every known name rather
/// than an empty list: an empty list reads as "this host offers nothing", which is a
/// different and wrong answer to a typo.
pub(in crate::mcp) fn render_catalogue(
    catalogue: &CommandCatalogue,
    detail: CatalogueDetail,
    filter: Option<&CommandName>,
) -> Value {
    let rows: Vec<&CommandEntry> = match filter {
        None => catalogue.entries.iter().collect(),
        Some(wanted) => catalogue
            .entries
            .iter()
            .filter(|entry| entry.command == *wanted)
            .collect(),
    };
    match filter {
        Some(wanted) if rows.is_empty() => {
            let known: Vec<&str> = catalogue
                .entries
                .iter()
                .map(|entry| entry.command.as_str())
                .collect();
            return tool_error(&format!(
                "no command named {:?} on host {:?}; it offers: {}",
                wanted.as_str(),
                catalogue.host.as_str(),
                known.join(", ")
            ));
        }
        Some(_) | None => {}
    }
    let rendered: Vec<Value> = rows
        .into_iter()
        .map(|entry| render_entry(entry, detail))
        .collect();
    text_content(&json!({
        "host": catalogue.host.as_str(),
        "detail": detail.label(),
        "commands": rendered,
    }))
}

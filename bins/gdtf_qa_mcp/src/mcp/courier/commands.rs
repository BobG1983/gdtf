use gdtf_qa_protocol::command::{CommandCatalogue, CommandEntry, CommandName};
use serde_json::{Value, json};

use crate::mcp::content::{text_content, tool_error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mcp) enum CatalogueDetail {
    Summary,
    Full,
}

impl CatalogueDetail {
    const fn label(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Full => "Full",
        }
    }

    pub(in crate::mcp) const ALL: [Self; 2] = [Self::Summary, Self::Full];

    fn from_label(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.label() == word)
    }

    pub(in crate::mcp) fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(Self::label).collect()
    }
}

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

pub(in crate::mcp) fn parse_filter(args: &Value) -> Result<Option<CommandName>, String> {
    match args.get("command") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => Ok(Some(CommandName::from_owned(name.clone()))),
        Some(other) => Err(format!("`command` must be a string, not {other}")),
    }
}

fn render_entry(entry: &CommandEntry, detail: CatalogueDetail) -> Value {
    let schemas = match detail {
        CatalogueDetail::Summary => Value::Null,
        CatalogueDetail::Full => json!({
            "arguments": entry.arguments.as_str(),
            "reply": entry.reply.as_str(),
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

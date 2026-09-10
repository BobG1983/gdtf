//! Measuring the build agent's output schema in the build workflow.

use std::{fs, path::Path};

/// The workflow that declares the schema, relative to the repo root.
pub(crate) const BUILD_TICKET: &str = ".claude/workflows/build-ticket.js";

/// The declaration whose object literal this guard measures.
pub(crate) const DECLARATION: &str = "const WORK_RESULT = {";

/// The largest schema a build may carry, in characters. Builds completed at 3215 and at
/// 3227; one at 4537 was refused before its agent did any work.
pub(crate) const LIMIT: usize = 3500;

/// What the build workflow says about the size of its schema.
pub(crate) enum Size {
    /// The literal's length in characters, opening brace through matching close.
    Chars(usize),
    /// The workflow does not declare it under that name.
    Missing,
    /// The declaration is there and its brace never closes.
    Unbalanced,
    /// The workflow could not be read.
    Unreadable,
}

impl Size {
    /// True only for a literal the guard measured and found within the limit.
    pub(crate) const fn is_within_limit(&self) -> bool {
        matches!(self, Self::Chars(chars) if *chars <= LIMIT)
    }

    /// What to print when this size fails the guard.
    pub(crate) fn report(&self) -> String {
        match self {
            Self::Chars(chars) => format!(
                "`WORK_RESULT` in {BUILD_TICKET} is {chars} characters, over the {LIMIT} \
                 this guard allows. Every build and fix agent carries this schema, and one \
                 of 4537 characters was refused with `output schema too large to classify \
                 safely` before the agent did any work. Take a field out, or shorten a \
                 description.",
            ),
            Self::Missing => format!(
                "no `{DECLARATION}` in {BUILD_TICKET}. That declaration is the build \
                 agent's output schema and this guard measures it by name, so renaming or \
                 deleting it turns the guard off. Point the guard at the new name.",
            ),
            Self::Unbalanced => format!(
                "`{DECLARATION}` in {BUILD_TICKET} never closes its brace, so the schema \
                 has no size to measure and the script cannot parse.",
            ),
            Self::Unreadable => format!("could not read {BUILD_TICKET}"),
        }
    }
}

/// Measure the schema in the build workflow under `root`.
pub(crate) fn measure_build_ticket(root: &Path) -> Size {
    fs::read_to_string(root.join(BUILD_TICKET)).map_or(Size::Unreadable, |text| measure(&text))
}

// The size of the object literal that opens on the declaration line.
fn measure(text: &str) -> Size {
    let Some(at) = text.find(DECLARATION) else {
        return Size::Missing;
    };
    let open = at + DECLARATION.len() - 1;
    text.get(open..)
        .and_then(literal_chars)
        .map_or(Size::Unbalanced, Size::Chars)
}

// Characters from the opening brace through its matching close, inclusive.
fn literal_chars(from_open: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, ch) in from_open.chars().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            _ if quote == Some(ch) => quote = None,
            _ if quote.is_some() => {}
            '\'' | '"' | '`' => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
    }
    None
}

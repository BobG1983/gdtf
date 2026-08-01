//! [`ToolName`] — the five tools, their listing order, and the wire-name mapping.

/// One MCP tool the courier exposes.
///
/// FIVE, and the set is closed. Three drive a child process — start it, stop it, read what
/// it printed — and two carry that child's own command layer: what it offers, and running
/// one of those things by name. Nothing here names a COMMAND, so a host gaining one changes
/// nothing in this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolName {
    /// Start a host's child process and wait for it to answer.
    Launch,
    /// Stop a host's child process.
    Stop,
    /// Read the tail of what a host's child printed to stdout or stderr.
    Logs,
    /// Read a host's live command catalogue — maps to `QaRequest::Catalogue`.
    Commands,
    /// Run one command from a host's catalogue by name — maps to `QaRequest::Run`.
    Run,
}

/// Every tool, in listing order — the one enumeration `tools/list` and any registry walk
/// read.
pub(super) const ALL: &[ToolName] = &[
    ToolName::Launch,
    ToolName::Stop,
    ToolName::Logs,
    ToolName::Commands,
    ToolName::Run,
];

impl ToolName {
    /// The MCP wire name a client calls this tool by.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Stop => "stop",
            Self::Logs => "logs",
            Self::Commands => "commands",
            Self::Run => "run",
        }
    }

    /// Resolve a `tools/call` name to its [`ToolName`], or `None` for an unknown tool.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }
}

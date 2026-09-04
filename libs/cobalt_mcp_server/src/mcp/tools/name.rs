//! Wire names for MCP tools.

/// Tools this bridge exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolName {
    /// Launch a host process.
    Launch,
    /// Stop a host process.
    Stop,
    /// Fetch child log tail.
    Logs,
    /// List QA commands from the host.
    Commands,
    /// Run a QA command on the host.
    Run,
}

pub(super) const ALL: &[ToolName] = &[
    ToolName::Launch,
    ToolName::Stop,
    ToolName::Logs,
    ToolName::Commands,
    ToolName::Run,
];

impl ToolName {
    /// Name as it appears on the wire.
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

    /// Parse a wire name.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolName {
        Launch,
        Stop,
        Logs,
        Commands,
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

        #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }
}

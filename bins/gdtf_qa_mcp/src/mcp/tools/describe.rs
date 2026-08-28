use crate::mcp::tools::name::ToolName;

impl ToolName {
    pub(super) const fn description(self) -> &'static str {
        match self {
            Self::Launch => {
                "Start a child process and wait for its QA channel to answer. `host` picks \
                 which one — \"game\" (the default) or \"editor\" — and everything else is \
                 optional: `port`, `package`, `features`, `working_dir` and `env` override \
                 that host's own defaults, so one call can build another package or another \
                 checkout. A child already running from the SAME recipe is kept and reported \
                 as already_running; one running from a DIFFERENT recipe is refused, naming \
                 what is actually up. The launched reply names the instance the child was \
                 recorded under; keep that id for `stop` and `logs`. Call `commands` next to \
                 see what that child offers."
            }
            Self::Stop => {
                "Stop a recorded child. `host` picks which one — \"game\" (the default) or \
                 \"editor\" — and `instance` names which of that host's recorded instances to \
                 stop, as the launch reply gave it. An editor call that names no instance \
                 while the host records any is refused, and the refusal lists every recorded \
                 editor instance. A child this MCP process did not start but that still holds \
                 the host's port — what an earlier session left behind — is stopped too, and \
                 reported as an orphan, so a reconnect never strands a live process."
            }
            Self::Logs => {
                "Read the tail of what a recorded child printed. Both of its output streams \
                 are captured into one buffer, in the order they were written, so this is \
                 what the process actually said. `host` picks which child — \"game\" (the \
                 default) or \"editor\"; `instance` names which of that host's recorded \
                 instances to read, as the launch reply gave it; `max_lines` caps how many \
                 trailing lines come back. An editor call that names no instance while the \
                 host records any is refused, and the refusal lists every recorded editor \
                 instance. Use it when a launch came up but the app is not behaving: the \
                 child's own log is usually the whole answer."
            }
            Self::Commands => {
                "Ask a running child what it can do. The reply is that host's live catalogue: \
                 one row per command with its name, a one-line summary, when it answers, and \
                 whether it can run RIGHT NOW in the state the host is in. `host` picks which \
                 child — \"game\" (the default) or \"editor\". `detail: \"Full\"` also \
                 returns each command's argument and reply shapes as RON text, which is what \
                 you read to build a `run` call. `command` narrows the reply to one row. Start \
                 every session here: the catalogue is the truth about what this build offers, \
                 and it changes with the host rather than with this tool."
            }
            Self::Run => {
                "Run one command from a host's catalogue. `command` is the name `commands` \
                 gave; `arguments` is a compact-RON string shaped by that command's own \
                 `schemas.arguments` (omit it for a command that takes none); `host` picks \
                 which child \
                 — \"game\" (the default) or \"editor\". Two optional riders: `await_ready` \
                 keeps re-testing admission for that many seconds instead of deciding once, \
                 and `capture` takes a screenshot after the command has run (true for a \
                 host-chosen file name, or a string to pick one). A command that cannot run \
                 right now, an argument it will not accept, and a name it does not know each \
                 come back with what you need to fix the call in one more round trip."
            }
        }
    }
}

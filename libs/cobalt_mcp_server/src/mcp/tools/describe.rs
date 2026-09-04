//! Tool descriptions, written from the hosts this server was started with.

use crate::{hosts::HostRegistry, mcp::tools::name::ToolName};

// How a description offers the `host` argument: every registered name, default first.
fn host_choices(hosts: &HostRegistry) -> String {
    let names = hosts.quoted_names();
    match names.split_first() {
        None => "no host is registered on this server".to_owned(),
        Some((first, [])) => {
            format!("{first} is the only registered host, so `host` may be omitted")
        }
        Some((first, rest)) => format!("{first} (the default) or {}", rest.join(" or ")),
    }
}

// Names of the hosts that keep several children at once, as prose lists them.
fn multi_instance_names(hosts: &HostRegistry) -> Option<String> {
    let names: Vec<String> = hosts
        .multi_instance_hosts()
        .iter()
        .map(|host| format!("{:?}", host.name().as_str()))
        .collect();
    (!names.is_empty()).then(|| names.join(" or "))
}

// Names of the hosts that keep one child, as prose lists them.
fn single_instance_names(hosts: &HostRegistry) -> Option<String> {
    let names: Vec<String> = hosts
        .single_instance_hosts()
        .iter()
        .map(|host| format!("{:?}", host.name().as_str()))
        .collect();
    (!names.is_empty()).then(|| names.join(" or "))
}

// The sentence a tool adds when a host can hold several children and the call must pick one.
fn instance_rule(hosts: &HostRegistry, verb: &str) -> String {
    multi_instance_names(hosts).map_or_else(String::new, |names| {
        format!(
            " A {verb} call on {names} that names no instance while the host records any is \
             refused, and the refusal lists every recorded instance."
        )
    })
}

fn launch_description(hosts: &HostRegistry) -> String {
    let many = multi_instance_names(hosts).map_or_else(String::new, |names| {
        format!(
            " A call on {names} starts another child every time, with its own fresh state, so \
             several can be up at once."
        )
    });
    let one = single_instance_names(hosts).map_or_else(String::new, |names| {
        format!(
            " On {names} a child already running from the same recipe is kept and reported as \
             already_running, and one running from a different recipe is refused, naming what is \
             actually up."
        )
    });
    format!(
        "Start a child process and wait for its QA channel to answer. `host` picks which one — \
         {choices} — and everything else is optional: `port`, `package`, `features`, \
         `working_dir` and `env` override that host's own defaults, so one call can build another \
         package or another checkout.{many}{one} The launched reply names the instance the child \
         was recorded under; keep that id for `run`, `commands`, `stop` and `logs`. Call \
         `commands` next to see what that child offers.",
        choices = host_choices(hosts),
    )
}

fn stop_description(hosts: &HostRegistry) -> String {
    format!(
        "Stop a recorded child. `host` picks which one — {choices} — and `instance` names which \
         of that host's recorded instances to stop, as the launch reply gave it.{rule} A child \
         this MCP process did not start but that still holds the host's port — what an earlier \
         session left behind — is stopped too, and reported as an orphan, so a reconnect never \
         strands a live process.",
        choices = host_choices(hosts),
        rule = instance_rule(hosts, "`stop`"),
    )
}

fn logs_description(hosts: &HostRegistry) -> String {
    format!(
        "Read the tail of what a recorded child printed. Both of its output streams are captured \
         into one buffer, in the order they were written, so this is what the process actually \
         said. `host` picks which child — {choices}; `instance` names which of that host's \
         recorded instances to read, as the launch reply gave it; `max_lines` caps how many \
         trailing lines come back.{rule} Use it when a launch came up but the app is not \
         behaving: the child's own log is usually the whole answer.",
        choices = host_choices(hosts),
        rule = instance_rule(hosts, "`logs`"),
    )
}

fn commands_description(hosts: &HostRegistry) -> String {
    format!(
        "Ask a running child what it can do. The reply is that host's live catalogue: one row per \
         command with its name, a one-line summary, when it answers, and whether it can run RIGHT \
         NOW in the state the host is in. `host` picks which child — {choices}.{rule} `detail: \
         \"Full\"` also returns each command's argument and reply shapes as RON text, which is \
         what you read to build a `run` call. `command` narrows the reply to one row. Start every \
         session here: the catalogue is the truth about what this build offers, and it changes \
         with the host rather than with this tool.",
        choices = host_choices(hosts),
        rule = instance_rule(hosts, "`commands`"),
    )
}

fn run_description(hosts: &HostRegistry) -> String {
    format!(
        "Run one command from a host's catalogue. `command` is the name `commands` gave; \
         `arguments` is a compact-RON string shaped by that command's own `schemas.arguments` \
         (omit it for a command that takes none); `host` picks which child — {choices}.{rule} Two \
         optional riders: `await_ready` keeps re-testing admission for that many seconds instead \
         of deciding once, and `capture` takes a screenshot after the command has run (true for a \
         host-chosen file name, or a string to pick one). A command that cannot run right now, an \
         argument it will not accept, and a name it does not know each come back with what you \
         need to fix the call in one more round trip.",
        choices = host_choices(hosts),
        rule = instance_rule(hosts, "`run`"),
    )
}

impl ToolName {
    pub(super) fn description(self, hosts: &HostRegistry) -> String {
        match self {
            Self::Launch => launch_description(hosts),
            Self::Stop => stop_description(hosts),
            Self::Logs => logs_description(hosts),
            Self::Commands => commands_description(hosts),
            Self::Run => run_description(hosts),
        }
    }
}

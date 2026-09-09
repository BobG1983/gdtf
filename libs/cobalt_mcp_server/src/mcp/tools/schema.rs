//! JSON schemas for `tools/list`, written from the registered hosts.

use serde_json::{Value, json};

use crate::{
    hosts::{HostRegistry, McpHostSpec},
    mcp::{
        courier::commands::CatalogueDetail,
        tools::name::{ALL, ToolName},
    },
};

// One row per registered host: its name, its port variable and its default port.
fn channel_table(hosts: &HostRegistry) -> String {
    let rows: Vec<String> = hosts
        .hosts()
        .iter()
        .map(|host| {
            format!(
                "{} reads {} and defaults to {}",
                host.name(),
                host.channel().port().as_str(),
                *host.default_port()
            )
        })
        .collect();
    rows.join("; ")
}

// "name: value" for every registered host, under a field each host supplies its own value for.
fn per_host_table(hosts: &HostRegistry, field: impl Fn(&McpHostSpec) -> String) -> String {
    let rows: Vec<String> = hosts
        .hosts()
        .iter()
        .map(|host| format!("{}: {}", host.name(), field(host)))
        .collect();
    rows.join("; ")
}

fn omit_clause(hosts: &HostRegistry) -> String {
    match hosts.default_host() {
        None => "no host is registered on this server".to_owned(),
        Some(first) if hosts.hosts().len() == 1 => format!(
            "{:?} is the only registered host, so this may be omitted",
            first.name().as_str()
        ),
        Some(first) => format!("omit for {:?}", first.name().as_str()),
    }
}

fn instance_clause(hosts: &HostRegistry) -> String {
    let names: Vec<String> = hosts
        .multi_instance_hosts()
        .iter()
        .map(|host| format!("{:?}", host.name().as_str()))
        .collect();
    if names.is_empty() {
        return " Each host runs one child, so no call needs an `instance`.".to_owned();
    }
    format!(
        " A host that keeps one child needs no `instance`; {} run as many as have been launched, \
         so a call on one of those names the instance it acts on.",
        names.join(" and ")
    )
}

fn host_property(hosts: &HostRegistry) -> Value {
    json!({
        "type": "string",
        "enum": hosts.names().iter().map(|name| name.as_str()).collect::<Vec<&str>>(),
        "description": format!(
            "Which host to act on; {}.{}",
            omit_clause(hosts),
            instance_clause(hosts),
        ),
    })
}

fn launch_schema(hosts: &HostRegistry) -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(hosts),
            "port": { "type": "integer", "minimum": 0, "maximum": 65535,
                      "description": format!(
                          "Loopback port for the child's mcp listener. The spawner sets each \
                           host's own port variable on the child it starts ({}). The launch \
                           probes the port given, so a child that binds a different one answers \
                           a 'did not answer within' error after the boot timeout. Omit it for \
                           that host's default.",
                          channel_table(hosts),
                      ) },
            "package": { "type": "string",
                         "description": format!(
                             "Cargo package to build and run; omit for that host's own ({}).",
                             per_host_table(hosts, |host| host.package().as_str().to_owned()),
                         ) },
            "features": { "type": ["array", "string"],
                          "items": { "type": "string" },
                          "description": format!(
                              "Cargo features to enable, as an array or a comma-separated \
                               string; omit for that host's own ({}).",
                              per_host_table(hosts, |host| host
                                  .features()
                                  .render()
                                  .unwrap_or_else(|| "none".to_owned())),
                          ) },
            "profile": { "type": "string",
                         "description": "Cargo profile to build and run under, e.g. \
                          \"release\"; omit for cargo's default dev profile. A release child \
                          still serves commands, because its QA channel is compiled in by a \
                          feature rather than by debug assertions." },
            "working_dir": { "type": "string",
                             "description": "Directory to run the build in — the \
                              checkout under test. Omit to use the MCP host's own; \
                              pass a git worktree path to QA that tree." }
        }
    })
}

fn instance_property(hosts: &HostRegistry) -> Value {
    json!({
        "type": "string",
        "description": format!(
            "Which recorded instance to act on, as the launch reply named it.{} A `run` or \
             `commands` call on such a host always names one, and a `stop` or `logs` call that \
             names none is refused while the host records any. Either refusal lists every \
             recorded instance.",
            instance_clause(hosts),
        ),
    })
}

fn stop_schema(hosts: &HostRegistry) -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(hosts),
            "instance": instance_property(hosts)
        }
    })
}

fn logs_schema(hosts: &HostRegistry) -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(hosts),
            "instance": instance_property(hosts),
            "max_lines": { "type": "integer", "minimum": 0,
                           "description": "How many trailing lines to return; omit for \
                            the default tail." }
        }
    })
}

fn commands_schema(hosts: &HostRegistry) -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(hosts),
            "instance": instance_property(hosts),
            "command": { "type": "string",
                         "description": "Narrow the reply to one command by name; omit \
                          for the whole catalogue. Names come from this tool." },
            "detail": { "type": "string",
                        "enum": CatalogueDetail::labels(),
                        "description": "How much of each row to return; omit for \
                         Summary. Full also carries each command's argument and reply \
                         shapes, as RON text." }
        }
    })
}

fn run_schema(hosts: &HostRegistry) -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string",
                         "description": "The command to run, as `commands` named it." },
            "arguments": { "type": "string",
                           "description": "The command's arguments as compact RON, shaped by \
                            its own `schemas.arguments` from `commands`; a command that takes \
                            none is \"()\"." },
            "host": host_property(hosts),
            "instance": instance_property(hosts),
            "await_ready": { "type": "integer", "minimum": 0,
                             "description": "Whole seconds to keep re-testing admission \
                              before giving up; omit to decide once." },
            "capture": { "type": ["boolean", "string"],
                         "description": "Capture the screen once the command has run: \
                          true for a host-chosen file stem, or a string to pick one." }
        },
        "required": ["command"]
    })
}

impl ToolName {
    fn input_schema(self, hosts: &HostRegistry) -> Value {
        match self {
            Self::Launch => launch_schema(hosts),
            Self::Stop => stop_schema(hosts),
            Self::Logs => logs_schema(hosts),
            Self::Commands => commands_schema(hosts),
            Self::Run => run_schema(hosts),
        }
    }

    fn descriptor(self, hosts: &HostRegistry) -> Value {
        json!({
            "name": self.wire_name(),
            "description": self.description(hosts),
            "inputSchema": self.input_schema(hosts),
        })
    }
}

/// MCP `tools/list` result listing every tool, with the registered hosts in each `host` enum.
#[must_use]
pub fn tools_list_result(hosts: &HostRegistry) -> Value {
    let tools: Vec<Value> = ALL.iter().map(|tool| tool.descriptor(hosts)).collect();
    json!({ "tools": tools })
}

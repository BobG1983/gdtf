//! The MCP tool registry — the tools this bridge exposes (GTW-741, GTW-745, GTW-749,
//! GTW-766, GTW-787, GTW-802, GTW-808, GTW-880, GTW-905).
//!
//! The [`ToolName`] enum is the single place the tool set is enumerated: `tools/list`
//! walks it, and [`ToolName::from_wire`] resolves a `tools/call` name. There is one tool
//! per variant, and `name`'s `ALL` slice lists them in advertised order.
//!
//! The set is [`TOOL_COUNT`] tools — [`FORWARDING_TOOL_COUNT`] forwarding and
//! [`HOST_LOCAL_TOOL_COUNT`] host-local. Those three counts are computed from `ALL` when
//! the crate compiles, so no number is written here to go stale; the two per-host lists
//! below and the two ports are re-derived from `ALL` by the
//! `the_module_doc_names_every_tool` test in `test`, which fails the suite the moment this
//! doc and the enum disagree — sibling tickets under GTW-786 keep changing the set, and a
//! doc nothing checks goes stale silently (this one claimed ten tools and one host for
//! months after GTW-802 and GTW-808).
//!
//! The bridge drives TWO child processes, not one: the game and the content editor
//! (GTW-808). [`ToolName::host`] is the wildcard-free `match` that partitions the set
//! between the two [`QaHost`](crate::hosts::QaHost) values, so a tool travels over its own
//! host's link rather than defaulting to the game's. A FORWARDING tool maps 1:1 onto a
//! [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest) sent to that running child; a
//! HOST-LOCAL tool never reaches a wire, because it starts or stops the child process
//! itself ([`ToolName::is_launch`] / [`ToolName::is_stop`]).
//!
//! `take_screenshot` is the one tool a call may aim at either child with a `host` argument
//! ([`ToolName::accepts_host_argument`], GTW-880); every other tool stays on the host
//! [`ToolName::host`] names.
//!
//! ## The `game` host — loopback port 7616
//!
//! Forwarding — [`SendInput`](ToolName::SendInput), [`QueryState`](ToolName::QueryState),
//! [`GetOutput`](ToolName::GetOutput), [`TakeScreenshot`](ToolName::TakeScreenshot),
//! [`ScreenshotAfter`](ToolName::ScreenshotAfter), [`AppFlow`](ToolName::AppFlow),
//! [`StartBattle`](ToolName::StartBattle), [`StepperControl`](ToolName::StepperControl),
//! [`ActivateMenuItem`](ToolName::ActivateMenuItem),
//! [`FocusControl`](ToolName::FocusControl).
//!
//! Host-local — [`LaunchGame`](ToolName::LaunchGame), [`StopGame`](ToolName::StopGame).
//!
//! ## The `editor` host — loopback port 7617
//!
//! Forwarding — [`GetEditorQueryOptions`](ToolName::GetEditorQueryOptions),
//! [`QueryEditor`](ToolName::QueryEditor).
//!
//! Host-local — [`LaunchEditor`](ToolName::LaunchEditor),
//! [`StopEditor`](ToolName::StopEditor).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `name` — the [`ToolName`] enum, the listing order, and the wire-name mapping.
//! - `count` — the [`ToolCount`] newtype and the three counts this doc quotes.
//! - `describe` — the per-tool human description a client reads to learn the workflow.
//! - `schema` — the per-tool JSON-Schema and the `tools/list` descriptor assembly.

mod count;
mod describe;
mod name;
mod schema;

#[cfg(test)]
mod test;

pub use count::{FORWARDING_TOOL_COUNT, HOST_LOCAL_TOOL_COUNT, TOOL_COUNT, ToolCount};
pub use name::ToolName;
pub use schema::tools_list_result;

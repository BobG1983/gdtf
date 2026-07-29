//! The parent module doc, pinned against `ALL` (GTW-905).

use crate::{
    hosts::QaHost,
    mcp::tools::{FORWARDING_TOOL_COUNT, HOST_LOCAL_TOOL_COUNT, TOOL_COUNT, name::ALL},
};

/// The doc's three counts, the variant set it names, its per-host lists and its two ports
/// are all tied back to `ALL` here, so the doc cannot drift from the enum.
///
/// The doc is where a reader learns what the bridge exposes, and a number written into it
/// goes stale the moment a tool is added — it claimed ten forwarding tools and one host
/// for months after GTW-802 and GTW-808 made it twelve and two. The counts are no longer
/// written into the prose at all: the doc links
/// [`TOOL_COUNT`](crate::mcp::tools::TOOL_COUNT) and its two siblings, which the compiler
/// computes from `ALL`. This test pins the four things the compiler cannot: that those
/// constants are still derived from `ALL` rather than replaced by literals, that the
/// sentence still links them instead of quoting numbers, that the variant set matches in
/// BOTH directions (so a variant deleted from `ALL` but left in prose fails too), that
/// each variant sits in the section of the host
/// [`ToolName::host`](crate::mcp::tools::ToolName::host) gives it, and that each port sits
/// inside its own host's section rather than merely somewhere in the doc.
#[test]
fn the_module_doc_names_every_tool() {
    const SOURCE: &str = include_str!("../mod.rs");
    let doc: String = SOURCE
        .lines()
        .filter(|line| line.trim_start().starts_with("//!"))
        .map(|line| line.trim_start().trim_start_matches("//!").trim())
        .collect::<Vec<&str>>()
        .join(" ");

    let host_local = ALL
        .iter()
        .filter(|tool| tool.is_launch() || tool.is_stop())
        .count();
    assert_eq!(*TOOL_COUNT, ALL.len(), "TOOL_COUNT counts `ALL`");
    assert_eq!(
        *HOST_LOCAL_TOOL_COUNT, host_local,
        "HOST_LOCAL_TOOL_COUNT counts the launch / stop tools in `ALL`"
    );
    assert_eq!(
        *FORWARDING_TOOL_COUNT,
        ALL.len() - host_local,
        "FORWARDING_TOOL_COUNT counts the rest of `ALL`"
    );
    let sentence = "The set is [`TOOL_COUNT`] tools — [`FORWARDING_TOOL_COUNT`] \
                    forwarding and [`HOST_LOCAL_TOOL_COUNT`] host-local.";
    assert!(
        doc.contains(sentence),
        "the module doc quotes the derived counts, not numbers: {sentence}"
    );
    assert!(
        !doc.contains("Ten tools"),
        "the stale ten-tool count is back: {doc}"
    );
    assert!(
        !doc.contains("a running game;"),
        "the stale game-only description is back: {doc}"
    );

    let mut named: Vec<String> = doc
        .split("ToolName::")
        .skip(1)
        .map(|tail| {
            tail.chars()
                .take_while(|char| char.is_alphanumeric() || *char == '_')
                .collect::<String>()
        })
        .filter(|ident| ident.starts_with(char::is_uppercase))
        .collect();
    named.sort();
    named.dedup();
    let mut expected: Vec<String> = ALL.iter().map(|tool| format!("{tool:?}")).collect();
    expected.sort();
    assert_eq!(named, expected, "the module doc names exactly `ALL`");

    let sections_end = doc.find("## Members").unwrap_or(doc.len());
    let mut marks: Vec<(QaHost, usize)> = QaHost::ALL
        .into_iter()
        .map(|host| {
            let marker = format!("## The `{}` host", host.label());
            let Some(at) = doc.find(&marker) else {
                unreachable!("the module doc carries a `{marker}` section");
            };
            (host, at)
        })
        .collect();
    marks.sort_by_key(|(_, at)| *at);
    for (index, (host, start)) in marks.iter().enumerate() {
        let end = marks.get(index + 1).map_or(sections_end, |(_, at)| *at);
        let Some(section) = doc.get(*start..end) else {
            unreachable!("the {host:?} section is a whole slice of the doc");
        };
        assert!(
            section.contains(&format!("{}", *host.default_port())),
            "the {host:?} section names its own port: {section}"
        );
        for tool in ALL.iter().copied() {
            let variant = format!("ToolName::{tool:?}");
            assert_eq!(
                section.contains(&variant),
                tool.host() == *host,
                "{variant} belongs to the {:?} section, not the {host:?} one",
                tool.host()
            );
        }
    }
}

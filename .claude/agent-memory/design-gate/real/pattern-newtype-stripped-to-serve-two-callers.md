---
name: pattern-newtype-stripped-to-serve-two-callers
description: A helper widened for a second caller tends to drop its newtype parameter for a bare &str while keeping its newtype return — the asymmetry is the tell.
metadata:
  type: feedback
---

When a diff widens an existing helper so a second wire type can use it, compare the
parameter list against the old signature. The cheap widening is to take the lowest common
denominator — `&str` — because the two feeding types are different newtypes with no shared
trait. The tell is asymmetry: the return stays a newtype, the parameter goes bare.

**Why:** `.claude/rules/no-bare-types.md` rule 1 covers function parameters, and its
Enforcement section allows a bare parameter only when it matches the enclosing newtype's own
inner type — `&str` against a `PathBuf` inner is not that. The shape that satisfies the rule
is in `bins/gdtf_qa_mcp/src/mcp/child_path.rs`: `resolve_child_path` takes
`ChildReportedPath<'_>` and returns `ChildFilePath` (`:44-47`), the wire type reaches it
through `From<&ArtifactPath>` (`:11-15`) instead of `.as_str()`, and the only caller passes
the newtype (`mcp/courier/attach.rs:11`). The helper exists to keep a path the CHILD reported
apart from a path this host built; with `&str` in that slot, a host path converted back to a
string type-checks — the exact confusion the two types prevent.

**How to apply:** when both callers strip their newtype at the call site, the remedy to ask
for is one shared newtype (or a small trait) that each wire type converts into, not a bare
parameter. Do not flag the bare `&str` params that never had a newtype to strip and return
none: `QaHost::from_label` (`bins/gdtf_qa_mcp/src/hosts/host.rs:38`) and
`ToolName::from_wire` (`mcp/tools/name.rs:41`) parse a wire word, and `tool_error`
(`mcp/content.rs:10`) takes output text.

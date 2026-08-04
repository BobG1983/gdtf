---
name: pattern-doc-recipes-must-be-executed
description: A clause demanding a reproducible doc recipe is failed by a command the implementer never ran in the doc's exact form — the tell is a flag present in the pasted evidence but missing from the doc.
metadata:
  type: feedback
---

When a clause says a doc must let a reader "reproduce clauses N-M", read the doc's commands
as executable text, not prose. Run each one in the exact form written, and compare the
result against what the doc says the reader will see.

**Why:** a recipe can be wrong in a way no reviewer spots by reading. The QA guide's port
check is the worked case: `docs/tooling/agent-qa.md:175` carries

```bash
lsof +c 0 -nP -iTCP:7617 -sTCP:LISTEN
```

and `:183` asserts "The `COMMAND` column must read `gdtf_content_editor`". Without `+c 0`,
macOS `lsof` truncates COMMAND to 9 characters, so a 19-character binary name prints as
`gdtf_cont` — indistinguishable from any other `gdtf_cont*` process, and the assertion the
doc makes is unreachable. `:178-181` is the paragraph explaining why the flag is
load-bearing; it exists because the first version of the recipe dropped the flag while the
implementer's own pasted evidence had it.

**How to apply:** diff the doc's commands against the evidence the implementer actually
pasted. A flag present in the evidence but missing from the doc is the tell, and it
generalises past `lsof` to any command whose output shape depends on a flag. Verifying an
OS tool's behaviour with a harmless local run (`lsof`, `ps`) is read-only measurement and is
allowed even under a zero-cargo gate.

Related: [[gate-manifest-guard-vacuity-check]], [[gate-untested-surfaces-inventory]].

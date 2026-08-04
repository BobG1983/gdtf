---
name: pattern-ticket-comment-is-a-snapshot
description: A ticket comment asserting a defect ("no test does that", "grep finds no reference") is frozen at its timestamp — re-run its own grep against the tree before ruling the action item unmet.
metadata:
  type: feedback
---

A ticket comment can carry an open action item stated as fact ("**No test does that.** grep
finds no other reference"). That is a snapshot of the branch at the comment's timestamp, not a
standing truth.

**Why:** one such comment demanded a false `build_command` doc comment be fixed before landing,
on the grounds that no test read back the composed command. By gate time the tree carried an
inline `mod tests` at `bins/gdtf_qa_mcp/src/lifecycle/spawn.rs:62-154` doing exactly what the
doc sentence describes — `get_args` (:74), `get_envs` (:81) and `get_current_dir` (:102), with
no launch (see `default_recipe_matches_plain_git_launch` at :88). The action item was closed by
code, not by an edit, and the implementer's summary never mentioned it. Ruling it a violation
off the comment alone would have been wrong.

**How to apply:** for every open action item quoted in a ticket comment, re-run the comment's
OWN grep or read against the worktree and cite the current file:line. Flag the reverse too — an
item satisfied incidentally deserves a note, so the board records why it is closed.

Related: [[pattern-partial-falsehood-sweep]].

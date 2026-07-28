# Module layout — a module is a DIRECTORY; mod.rs is wiring-only; files stay small

Why this rule exists: GTW-583's census found 117 files past the 400-line block
line and 4 logic-bearing mod.rs files. A monolith file lumps unrelated
change-reasons, so every edit collides with every other and review/bisect decay.
One concern per file, wiring separated from logic, keeps each change small and
each file readable in one sitting.

## Rules

1. **A module is a DIRECTORY.** A concern that outgrows one file becomes a
   directory module: `<name>/mod.rs` plus focused per-concern submodules. Split
   by CHANGE-REASON/CONCERN — "what makes this code change together" — never by
   mechanical halving.
2. **mod.rs is WIRING-ONLY — a mod.rs contains NO fns.** Permitted contents:
   the module doc, `mod` declarations, and re-exports (`pub use` preserving the
   module's public paths). NOTHING else: no fn of ANY kind — not a helper, not
   a `register_*(app: &mut App)` aggregator, not even a delegating
   `Plugin::build` (the fn-form aggregation ruling was REJECTED, user ruling
   2026-07-04; plugin aggregation lives in a sibling leaf file such as
   `plugin.rs` / `register.rs` / `sim_acts_plugin.rs`, re-exported from the
   mod.rs) — no impl, no type definitions, no closure systems inside
   `add_systems`. Exceptions go through the recorded exemption registry only.
3. **Line bands cover ALL THREE BANDS** (Q9 ruling, user-confirmed 2026-07-04):
   (a) logic files, (b) in-src test modules (`test.rs`/`tests.rs`, `test/` or
   `tests/` dirs under src, `test_*` basenames, `test_support`), and
   (c) integration files under `crates/*/tests/`.
   **WARN over 300** lines — split at the next change-reason boundary before
   landing more growth. **BLOCK over 400** lines — must not land; the clause-7
   conformance test (`crates/gdtf_test_utils/tests/module_layout/`) fails the
   suite.
4. **Crate roots** (`lib.rs`/`src/main.rs`) follow the same bands unless they
   are PURE WIRING (docs + `mod`/`pub mod` decls + re-exports, zero fn/impl) —
   a pure-wiring crate root is the crate-level equivalent of a long wiring
   mod.rs and is exempt via the registry (e.g. `gdtf_battle_sim/src/lib.rs`).
5. **Test placement** follows the GTW-583 test convention: sibling `test/`
   directory for unit tests; `tests/<suite>/main.rs` dir-form for integration
   suites; an inline `#[cfg(test)] mod test` only while tiny (see the
   decomposition record / TEST CONVENTION).
6. **Splits are behavior-preserving pure moves.** The only edits allowed
   beyond the move: visibility (`pub(super)`/`pub(crate)`/`pub(in path)`),
   import paths, and intra-doc-link re-pathing (never link deletion — the
   `cargo doc` gate with `broken_intra_doc_links = deny` is the backstop).
   Never invent shared abstractions to shrink counts; never move logic out of
   `gdtf_battle_sim` into `gdtf_battle_presenter` or back the other way;
   helpers with 2+ consuming modules live in the shared support/harness
   module, single-consumer helpers stay local to their consumer.
7. **A module's `pub use` may lift only from its own DESCENDANTS.** A module
   re-exports its own submodules' items — never a sibling's, a cousin's, or
   another family's (`pub use crate::other_family::…` presented as this
   module's API). A cross-family re-export erases a concern split at the
   public surface: consumers import the type via the lying path and the
   families read as one (GTW-624 — `equipment::weapon` re-exported the entire
   attachments palette + mechanics, erasing the GTW-558 split for 20+
   consumers). Consumers import a type from its owning family's true path.
   The crate root (`lib.rs`) is exempt — every module is its descendant.

## The census command (pinned)

Byte-reproducible; run from anywhere; classification precedence: mod.rs
basename first, then `crates/*/tests/` (integration), then in-src test
patterns, else logic. Counts are raw `wc -l`; ordering is (-lines, path).

```bash
python3 - <<'PYEOF'
import subprocess, re, os
R = "<repo root>"
out = subprocess.run(["git","-C",R,"ls-files","--","crates","bins"],capture_output=True,text=True,check=True).stdout
files = sorted(p for p in out.splitlines() if p.endswith(".rs"))
bands = {"logic":[], "src-test":[], "integration":[], "mod":[]}
for p in files:
    base = p.rsplit("/",1)[-1]
    if base == "mod.rs": bands["mod"].append(p)
    elif re.match(r"^crates/[^/]+/tests/", p): bands["integration"].append(p)
    elif re.search(r"/tests?\.rs$", p) or re.search(r"/tests?/", p) or base.startswith("test_") or "test_support" in p: bands["src-test"].append(p)
    else: bands["logic"].append(p)
def nlines(p):
    with open(os.path.join(R,p),"rb") as f: return sum(1 for _ in f)
BLOCKC = re.compile(r"/\*.*?\*/", re.S)
def modrs_logic(p):
    src = open(os.path.join(R,p),encoding="utf-8").read()
    src = BLOCKC.sub("", src)
    src = "\n".join(re.sub(r"//.*","",l) for l in src.splitlines())
    reasons = []
    fns = re.findall(r"\bfn\s+(\w+)", src)
    bad_fns = fns
    if bad_fns: reasons.append("fn " + ",".join(sorted(set(bad_fns))))
    impls = re.findall(r"\bimpl\b[^{;]*", src)
    bad_impls = [i.strip() for i in impls if not re.search(r"\b(Plugin|PluginGroup)\s+for\b", i)]
    if bad_impls: reasons.append("impl: " + "; ".join(bad_impls))
    if re.search(r"add_systems\s*\([^)]*\|", src): reasons.append("closure-system in add_systems")
    return reasons
for b in ("logic","src-test","integration"):
    sized = [(p,nlines(p)) for p in bands[b]]
    block = sorted([x for x in sized if x[1]>400], key=lambda x:(-x[1],x[0]))
    warn  = sorted([x for x in sized if 300<x[1]<=400], key=lambda x:(-x[1],x[0]))
    print(f"\n== {b}: total={len(sized)}  BLOCK(>400)={len(block)}  WARN(301-400)={len(warn)}")
    for p,n in block: print(f"BLOCK {n:5d} {p}")
    for p,n in warn:  print(f"warn  {n:5d} {p}")
flagged = [(p,nlines(p),modrs_logic(p)) for p in bands["mod"]]
flagged = [(p,n,r) for p,n,r in flagged if r]
print(f"\n== mod.rs: total={len(bands['mod'])}  logic-bearing={len(flagged)}")
for p,n,r in sorted(flagged, key=lambda x:(-x[1],x[0])): print(f"MODLOGIC {n:5d} {p}  [{'; '.join(r)}]")
PYEOF
```

(The fn allowlist in `modrs_logic` is EMPTY — Rule 2's "no fns" per the
2026-07-04 user ruling that rejected the fn-form aggregation carve-out:
`build`/`name`/`register_*` were all removed. The remaining `Plugin|PluginGroup
for` impl allowance is inert in practice — any such impl carries a `fn build`
and is flagged by the fn check — and is kept only so this detector matches the
guard's exactly.)

## Exemption registry

`.claude/rules/module-layout-exemptions.txt` — one entry per line:
`<repo-relative-path> — <one-paragraph why> (approved: GTW-N, <date>)`.
Lines starting with `#` are comments.

- A file that genuinely cannot split without harming cohesion is PROPOSED with
  a one-paragraph why and USER-APPROVED before the entry is added — never
  silently registered, never silently dropped.
- The conformance test honors the registry and FAILS on stale entries (a
  registered path that no longer exists or no longer violates) — the registry
  only ever shrinks or is deliberately re-approved.
- **Current approved entries: NONE.** The registry holds comments only. Every
  exemption it has ever carried was retired by fixing the file rather than by
  keeping the carve-out: `crates/gdtf_battle_sim/src/lib.rs` went when GTW-628
  deleted the crate-root flat name ledger, and the two `gdtf_ui` widgets
  (`text_field.rs` / `dropdown.rs`) went when GTW-655 retired `procgen_viz` and
  deleted both files outright. So the registry currently hides nothing — any
  file over the 400-line limit is a genuine violation the guard did not catch,
  not an approved one.

## Enforcement

Clause-7 conformance test: `crates/gdtf_test_utils/tests/module_layout/`
(dir-form suite, target `module_layout`) walks the tracked tree on every
`cargo dtest` run — any block-band file or
logic-bearing mod.rs not in the registry fails the suite, loudly, with the
census-format violation lines. `/gate`'s design-gate structure lens audits
splits for change-reason quality (this rule's Rule 1/6), which the line-count
guard cannot see.

---
name: gate
description: >-
  Verify an implementation against its ticket contract before landing. Pulls the
  Linear ticket(s), restates them as a clause-numbered contract, runs the suite, and has
  a design-gate sub-agent verify the diff first-hand. Use after implementing a
  ticket (or a runtime-coupled set that must land together) and before /land.
argument-hint: "[GTW-N ...]"
---

# /gate — contract check before landing

Why this exists: agents quietly **narrow** user-specified designs to "easy over
right" and claim completion without running anything. The gate freezes the ticket as
a contract and has an independent design-gate sub-agent check the code against it.
Binding background: `.claude/rules/design-fidelity.md`, `.claude/rules/verification.md`,
and the design canon under `docs/` (pillars, combat, glossary, litmus-tests, ADRs) —
that canon is part of the contract ALONGSIDE the ticket.

## Steps

1. **Resolve the ticket(s).** The `GTW-N` argument may name a single ticket OR a
   space/comma-separated SET of tickets (for runtime-coupled work — e.g. GTW-2 + GTW-3
   — that must land together); if absent, derive the single ticket from the branch name
   (`feature/gtw-N-slug`) and confirm with the user. Single-ticket is the common path.
   Pull the FULL ticket for EACH named id via the Linear MCP — project **GDTF**; discover
   the owning team through the MCP (the team that owns project GDTF), do NOT hardcode a
   team name. Read title, description, acceptance criteria, comments. Never gate from
   memory of the ticket.
2. **Run the suite — green is mandatory at gate time.** The one definition of green,
   from the repo root (`$CLAUDE_PROJECT_DIR`):

   ```bash
   cargo fmt --check
   cargo dclippy
   cargo dtest
   cargo dbuild
   cargo doc --workspace --no-deps
   cargo doc-full
   ```

   **Use these ALIASES, never hand-typed long-form flags** — `dclippy`/`dtest`/`dbuild`
   are aliases defined in `.cargo/config.toml` for `clippy`/`test`/`build` with the
   `dynamic_linking` feature enabled, plus `dev_tools` and BOTH `net_qa` features
   (`grimdark_turfwar/net_qa` for the game's module, `gdtf_content_editor/net_qa` for the
   editor's — a package-qualified feature turns on only that package's, so both are named).
   Typing the equivalent flags by hand risks silently dropping a feature and
   falling back to a slow, fully static rebuild, or leaving a feature-gated module dark —
   exactly the drift this instruction exists to prevent. `doc-full` is the same command
   with `dev_tools` + both `net_qa` features enabled, catching intra-doc-link regressions
   inside those feature-gated modules that plain `doc` alone would miss.
   Green = all SIX exit 0. `cargo doc`/`doc-full` enforce `broken_intra_doc_links` /
   `private_intra_doc_links` = `"deny"` (only surface under `cargo doc`, not
   clippy/build/test). The workspace `Cargo.toml` denies clippy
   all/pedantic/correctness plus unwrap/expect/panic/todo/unimplemented and
   missing_docs, so fmt-clean and lint-clean ARE part of green. Red → the gate FAILS
   immediately; fix the suite before anything else.
   Steps 4a–4d add BLOCKING structural checks (insufficient tests, unwired
   systems/plugins, oversized files, no leftover ticket ids) that fail the gate like a clause violation and
   route through the same step-7 repair loop. These are JUDGMENT checks scoped to the
   diff — they live HERE and in the design-gate reviewers, NOT the pre-commit hook,
   which stays a deterministic branch/gate-pass/suite backstop and makes no design call.
   So they bind only when `/gate` is actually run: skipping `/gate` leaves no backstop
   for them (the hook can't read a ticket contract).
3. **Restate the contract as clause-numbered** (C1, C2, …): one clause per
   requirement / acceptance criterion, faithful to the ticket's wording and to any
   `docs/` canon it invokes — do not soften, merge, or drop clauses. When MULTIPLE
   tickets were named, restate ALL of their contracts (the combined clause set covers
   every named ticket); attribute each clause to its source ticket. Sim behavior
   (gdtf_battle_sim) clauses must name the observable outcome and the test that proves
   it. From here the contract is **read-only**; it is what the code is measured
   against, not a draft to negotiate with.
4. **Gather the diff.** Capture, including untracked files:

   ```bash
   git status
   git diff develop...HEAD
   git ls-files -o --exclude-standard      # untracked, so the gate sees new files
   ```

   (Use `git diff develop...HEAD` when commits exist on the branch; otherwise
   `git diff HEAD`. Untracked files are part of the change and must be reviewed.)
4a. **Blocking check — insufficient tests (real-path coverage).** Make `verification.md`
   Rule 2 a hard gate: EVERY behavioral clause — new or changed logic, in ANY crate —
   MUST map to a test on the REAL code path that ASSERTS on the change, not a fixture
   echo. The grep below is only a cheap candidate-finder for ONE crate; absence of hits
   is NOT clearance — enumerate every behavioral clause across all crates and confirm a
   real-path, assertion-bearing test per clause by reading it. The bar differs by crate
   (`verification.md` Rules 2–3): a sim clause (`crates/gdtf_battle_sim`) needs a unit
   test with injected seeded RNG; a presenter/scene/view clause needs a headless Bevy
   integration test OR observed in-engine evidence from
   `cargo drun`. Candidate-finder:

   ```bash
   # sim fns added/changed in the diff (skip doc/comment lines), then any #[test] for them
   git diff develop...HEAD -- crates/gdtf_battle_sim | grep -nE '^\+[^/]*\b(pub|pub\(crate\)|fn) ' | grep -v '^\+ *//'
   grep -rn '#\[test\]' crates/gdtf_battle_sim
   ```

   A behavioral clause with no real-path, assertion-bearing test = **VIOLATION**. A test
   that calls the unit with no assertion, asserts on its own setup, or would still pass
   with the clause reverted is ITSELF a violation — the pin-discrimination check (*would
   this test fail if the clause were violated?*) is MANDATORY, not optional. A pure
   refactor, a rename, a visibility-only change (`pub(crate)`→`pub`), a signature move, or
   a docs-only clause adds no behavior and is exempt — but ONLY when the ticket says so.
   A trivially green `cargo test` does NOT clear this check. **Brittle tunable-data tests:** a
   test asserting an EXACT MAGNITUDE of a value tunable by definition (combat-tuning coefficients /
   band edges / body-part weights in `docs/combat/`, or theme-style data) is a VIOLATION even with
   a test-local fixture literal — it locks a number meant to be tuned and mostly re-tests serde;
   require parse-OK of the SHIPPED data file, round-trip IDENTITY, or a consistency invariant
   instead. Coordinate-system CONSTANTS a ticket requires pinned (e.g. the battle-space metric
   180/170/8) are exempt — system definition, not balance tuning.
4b. **Blocking check — unwired systems/plugins (the dead-code footgun).** A Bevy system
   fn, `*Plugin`, resource, message/event, reflected type, or state that is AUTHORED but
   never registered never RUNS — the Bevy analogue of dead code, silently narrowing a
   clause to an unreachable system. Use the FULL `health-check` step-2 wiring set (it is
   broader than a name-grep): a system fn must reach the App via a scene `plugin.rs`
   `add_systems(OnEnter/OnExit/Update/FixedUpdate, …)`, that `*ScenePlugin` via
   `scenes/plugin.rs` `add_plugins(…)`, and `ScenesPlugin` via `app/gdtf_app.rs`. Also
   check `init_resource`/`insert_resource`, `add_message`/`add_event`, `register_type`,
   `init_state`, run-condition closures (`run_if`/`in_state`/`resource_exists`), label/
   asset strings, and a `Plugin::build()` in ANY crate that may register (incl.
   `gdtf_battle_*`). Mechanism (per new symbol):

   ```bash
   grep -rnE 'add_systems|add_plugins|init_resource|insert_resource|add_message|add_event|register_type|init_state|run_if|in_state|resource_exists|fn build' crates/*/src
   ```

   A grep MISS is NOT an auto-fail: indirect registration (a `SystemSet`, a helper taking
   the schedule, a sub-plugin's `build()`) can wire it — confirm by READING the
   registration path before condemning, or you false-fail live code. Rule: a clause
   CLAIMING a system/plugin/state runs that is genuinely unwired = **VIOLATION**. An
   unwired scaffold the ticket does NOT claim live is FINE when `docs/` specifies the
   scene as built-ahead/dormant — but the exemption must CITE the `docs/` line
   (`health-check` step-4 KEEP rule, `design-fidelity.md`). Remember a resource needs an
   OnEnter/OnExit pair + a `run_if`/`Option<Res<…>>` guard (`bevy-traps.md` #1).
4c. **Blocking check — oversized files (count + cohesion).** List every `.rs` SOURCE the
   diff touches or adds — tracked AND untracked, dropping deleted paths — and `wc -l`
   each existing one (exclude generated/asset files; today's tree tops out at 47 lines,
   so this has wide headroom). PROPOSED threshold (an editorial call, not user-given —
   see open questions): **warn > 300 lines, BLOCK > 400 lines.** A line count alone
   cannot tell a cohesive large file from a bloated one, so an over-cap file is a
   **VIOLATION only when the reviewer also confirms it mixes unrelated responsibilities**
   — split it along the small-focused-systems idiom (`CLAUDE.md` Conventions,
   `bevy-traps.md`). A legitimately-large cohesive file (a data table, an exhaustive match)
   passes when the ticket sanctions it. A warn-band file is reported, not failed.
   Mechanism:

   ```bash
   { git diff develop...HEAD --name-only -- '*.rs'; git ls-files -o --exclude-standard -- '*.rs'; } \
     | sort -u | while read -r f; do [ -f "$f" ] && wc -l "$f"; done   # >400 blocks, >300 warns
   ```
4d. **Blocking check — no leftover ticket ids, comment hygiene.**
    No `GTW-N` (or any ticket id) ANYWHERE in the tree — comments, asserts, docs,
    assets, configs, the lot. Any hit fails the gate.
    No banned jargon from `plain-language.md` in comments.
    Doc comments (`///` / `//!`) max 2 lines; longer = violation unless the
    ticket explicitly requires a longer contract note.
    Mechanism (run from repo root):

    ```bash
    # ANY remaining GTW- string fails the gate — empty output required
    rg -n 'GTW-' --glob '!target/**' --glob '!.git/**' .
    # banned words in comments
    rg -n --type rust -i '^\s*//.*(seam|byte[- ]identical|sanctioned|leverage|canonical)\b' crates bins
    # doc blocks longer than 2 lines (3+ consecutive)
    rg -U --type rust -n '^(//!|///).*\n(//!|///).*\n(//!|///)' crates bins
    ```

    If the first `rg` prints anything, the gate FAILS. Strip every hit before re-gating.

5. **Verify the diff first-hand via the design-gate workflow.** This is a workflow
   orchestration step, not a standing team. **Default: a 3-lens adversarial fan-out** —
   in a SINGLE message the orchestrating session spawns THREE parallel read-only
   **design-gate** sub-agents (the same fan-out shape `health-check` step 2 uses), each
   briefed to one lens so the lenses cannot all share one blind spot and each MUST cite
   lens-specific evidence:
   - **Fidelity lens** — every clause vs. the contract's exact words + `docs/` canon;
     hunt quiet narrowing and hedge markers (`design-fidelity.md`).
   - **Tests lens** — check 4a: every behavioral clause has a real-path, assertion-bearing,
     pin-discriminating test (`verification.md` Rules 2–3), and NO brittle exact-magnitude
     assertion on tunable data (the 4a brittle-tunable-data rule).
   - **Structure/Bevy lens** — checks 4b + 4c, the `no-bare-types.md` rules (every domain value a
     named newtype INCLUDING leaf fields inside a grouping struct; newtype inner field PRIVATE and
     `Deref` DERIVED — not hand-impl'd — per `crates/gdtf_ui/src/theme.rs`; no direct dep on a
     crate the framework re-exports, e.g. `glam` vs `bevy::math`), the `module-layout.md` Rule 7
     re-export gate (a module's `pub use` may lift only from its own DESCENDANTS — a re-export of
     a sibling/cousin/other-family path as this module's API is a VIOLATION; crate roots exempt),
     and the `bevy-traps.md` ECS
     traps (unwired wiring chain, file size+cohesion, ambiguous ordering, OnEnter-without-OnExit
     resources, `EventWriter` vs `MessageWriter`).
   **Lightweight option:** when a single pass is enough, spawn ONE design-gate sub-agent
   carrying all three lenses (it already encodes 4a–4c). Either way pass the ticket id(s),
   the numbered contract (including 4a–4d), the touched `docs/` paths, the diff, and this
   instruction verbatim: "Verify every clause FIRST-HAND — read the code and run what you
   must (including `cargo drun` or a headless
   Bevy integration test for runtime behavior); do not trust the implementer's claims or
   this summary; also enforce the test-sufficiency, unwired-systems, file-size, and no-GTW
   checks (4a–4d) and run the green suite yourself." When MULTIPLE tickets were named, the
   reviewers audit the ONE combined diff against the combined contract (every clause of
   every named ticket) in a single pass. Reviewers are READ-ONLY and report back to the
   spawning session; they do not spawn further sub-agents. **Merge ANY-NON-COMPLIANT-
   BLOCKS:** one NON-COMPLIANT (or one dead/empty reviewer — uncertainty never favors the
   implementer) fails the whole gate. In the relay (step 6) name WHICH lens failed and
   why, so a reviewer that died is distinguishable from a real violation.
6. **Relay the verdict** to the user: per-clause PASS / VIOLATION with the sub-agent's
   evidence. Do not editorialize a VIOLATION into a pass.
7. **On violations, repair the CODE.** Fix each violated clause in the implementation
   (Rust crates under `crates/`, the binary under `bins/`). **Forbidden:** editing the
   contract, deleting or weakening acceptance criteria, or narrowing the design so the
   check passes — that is the exact failure this gate exists to stop. If you believe a
   clause itself is wrong, STOP and ask the user; never self-amend the contract. If
   fixing a violation properly looks **"too expensive"**, that is a USER decision —
   present the tradeoff and STOP; never engineer around it, never quietly re-scope.
8. **Re-gate after repairs**: repeat steps 2–6 against the SAME contract. **Maximum 2
   repair rounds.** If violations remain after the second re-gate, STOP — report the
   remaining violations to the user, leave the ticket open, and do not proceed to /land.
9. **On full PASS, record it for /land** — write ONE `.claude/.gate-pass` (gitignored via
   `.claude/.gitignore`, so writing it cannot disturb the fingerprint it records;
   never stage it). With multiple tickets, the single `TICKET=` line lists them all,
   comma-separated (e.g. `TICKET=GTW-2,GTW-3`):

   ```bash
   { printf 'TICKET=%s\n' "GTW-N"        # comma-separated for a multi-ticket set
     printf 'BRANCH=%s\n' "$(git branch --show-current)"
     printf 'HEAD=%s\n' "$(git rev-parse HEAD)"
     printf 'FINGERPRINT=%s\n' "$({ git rev-parse HEAD; git status --porcelain; git diff HEAD; git ls-files -o --exclude-standard -z | LC_ALL=C sort -z | xargs -0 -r shasum -a 256; } | shasum -a 256 | cut -d' ' -f1)"
   } > .claude/.gate-pass
   ```

   The fingerprint covers HEAD, all tracked changes, AND the **content of every
   untracked file** (`git ls-files -o`) — an all-untracked tree is fully covered.
   The pre-commit hook reads this file too: without it (or with a stale BRANCH/HEAD)
   every `git commit` is blocked. The hook and `.claude/.gitignore` are this skill's
   companions — `/gate` writes the fingerprint, the hook enforces it, `/land` consumes
   and clears it.
10. **Move every named ticket to In Review** via the Linear MCP (project **GDTF**, team
    discovered via MCP) — for a multi-ticket set, move ALL of them — statuses move with
    the work (`.claude/rules/linear-discipline.md`). Then report PASS and point the user
    at `/land`. /gate NEVER moves a ticket to Done — only landing does.

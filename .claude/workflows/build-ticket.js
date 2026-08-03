export const meta = {
  name: 'build-ticket',
  description: 'Build one GTW ticket end to end: open it, build, verify, 3-lens gate with a bounded fix loop, land, close',
  phases: [
    { title: 'Open', detail: 'fetch the ticket verbatim and mark it In Progress' },
    { title: 'Build', detail: 'implement against the quoted contract' },
    { title: 'Verify', detail: 'full green suite plus the ticket evidence clauses' },
    { title: 'Gate', detail: '3 read-only lenses, any-non-compliant blocks' },
    { title: 'Fix', detail: 'bounded repair loop on a red verdict' },
    { title: 'Land', detail: 'commit, finish, push, close with evidence' },
  ],
}

// args: { ticket: "GTW-123", slug: "editor-net-qa-passthrough" }
// Accept either a real object or a JSON-encoded string — the harness may deliver either.
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { throw new Error(`args was an unparseable string: ${A}`) }
}
const TICKET = A?.ticket
const SLUG = A?.slug
if (!TICKET || !SLUG) throw new Error(`args must supply { ticket, slug }; got ${JSON.stringify(args)}`)

// Optional orchestrator notes: findings from the clause audit that ran BEFORE this workflow was
// launched. They are readings OF the contract, never additions to it — the ticket still wins on
// anything they contradict. They reach the build and the fix agent only; the reviewers judge the
// ticket text alone, so a wrong note cannot talk a lens into passing something.
const NOTES = typeof A?.notes === 'string' && A.notes.trim() ? `
## ORCHESTRATOR NOTES — from the clause audit already run on this ticket

These are readings of clauses the audit found easy to get wrong. They do not add scope. If one
appears to contradict the ticket text above, the TICKET wins — and say so in your report.

${A.notes.trim()}
` : ''

const REPO = '/Users/bgardner/dev/gdtf'
const BRANCH = `feature/${SLUG}`
// Worktrees live under .claude/worktrees/ because that is the location Claude Code expects: it is
// already listed in .git/info/exclude, and EnterWorktree prompts the USER to approve a
// permission-root relocation for any worktree outside it — which stalls an autonomous run until a
// human notices. A `Bash(git worktree *)` allow rule does NOT suppress that prompt; it comes from
// the EnterWorktree tool, not from a shell command.
const WT = `${REPO}/.claude/worktrees/${SLUG}`

const HOUSE_RULES = `
## Standing rules — these override convenience and they are not negotiable

1. **NEVER drive the app with a script.** No Python client, no raw \`TcpStream\`, no \`nc\`, no shell
   loop speaking the wire protocol, no OS automation (AppleScript, screen capture). Drive the app
   ONLY through the \`mcp__gdtf-qa__*\` tools.
   Reading a process's stderr, or checking a port with \`lsof\`, is NOT driving the app and is fine.

   **If a tool you need is missing from the catalogue, or the tools are stale: STOP AND REPORT.
   That is the whole procedure.** Say which tool is missing and what you were trying to do. Only
   the USER can fix it, by running \`/mcp\`.
   - **Do NOT kill the resident process.** That was tested on 2026-07-28: killing it does NOT
     respawn the server, it removes every \`mcp__gdtf-qa__*\` tool from the session until the user
     reconnects. You would make it strictly worse.
   - **Do NOT rebuild \`gdtf_qa_mcp\` by hand.** \`.mcp.json\` spawns the server as
     \`sh -c "cargo build -p gdtf_qa_mcp && exec target/debug/gdtf_qa_mcp"\`, so the user's \`/mcp\`
     rebuilds it. Your rebuild changes nothing about what is connected.
   - **Do NOT build a bypass, do NOT engineer a mechanism to capture the evidence another way, and
     do NOT defer the clause into a new ticket.** Every one of those has been tried on this epic
     and every one was wasted work. Stopping and reporting costs one message.

6. **Pick the CHEAPEST SUFFICIENT evidence** (user ruling, 2026-07-29):

       integration test  ≈  unit test  >  reading the code  >>  building MCP tooling to "test"

   Tests are the BEST evidence, not a fallback. A test that runs the real plugins over a real
   listener is NOT a stand-in — it is often STRONGER than driving a spawned process, because it is
   deterministic where the live version is a race. **Before accepting any "prove it live" clause,
   READ THE EXISTING TESTS**; if one already covers it, say so and move on. If a clause's evidence
   is a race to capture, the clause is wrong — stop and report rather than engineering around it.
   GTW-902 burned five gate rounds, a bespoke readout mechanism and 20,000 fabricated asset files
   learning this.

7. **Never make a CI run URL part of your evidence.** \`gh\` on this machine is permanently
   attached to the user's WORK account and returns HTTP 404 for this personal repo; the ticket to
   fix it was cancelled as unfixable. Prove CI behaviour by pinning the workflow files with a test
   (see \`crates/gdtf_test_utils/tests/ci_workflow_features/\`), never by observing a run.
2. **Prove behaviour against the REAL binary.** A test built on \`MinimalPlugins\` + a hand-inserted
   resource proves nothing about the game or the editor. This epic already shipped two tickets
   "verified" that way and one asserts something false about the real editor.
3. **Verify every claimed dependency against the code.** A note in a ticket, a memory, or a status
   file is not evidence. This board carried "805 → 806 → 808 in strict order, each blocks the
   next" for days; it was assumption, never checked, and false.
4. **No \`unwrap\`/\`expect\`/\`panic\`/\`todo\`/\`unimplemented\`.** Doc every \`pub\` item. Typed domain
   values — no bare primitives (\`.claude/rules/no-bare-types.md\`). Files: warn over 300 lines,
   block over 400. \`mod.rs\` is wiring only, no fns.
5. **Plain language.** Banned words anywhere — tickets, code, comments, commit messages:
   "seam", "sanctioned", "byte identical"/"byte-identical". Name the real mechanism.
6. **Report failures verbatim.** Paste the failing assert, compiler or clippy output. Never
   summarise a failure away, never present partial success as success.
`

const GREEN = `
The one definition of green — run from the repo root, ALL EIGHT must pass, judged by EXIT CODE:
\`\`\`
cargo fmt --check
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
cargo doc --workspace --no-deps
cargo doc-full
cargo clippy-schema -- -D warnings
cargo test-schema
\`\`\`
Run them SEQUENTIALLY, one command per tool call. Never background them, never chain them with
\`&&\` into one call, never build a sentinel-file wait loop.

## PIN THE DIRECTORY ON EVERY SINGLE CALL — a bare \`cd\` does NOT persist

**The Bash tool's working directory resets to \`${REPO}\` between calls.** A \`cd ${WT}\` that
succeeds in one call is GONE by the next one, which then runs in the MAIN repo on \`develop\` —
against code that is not yours. That run can report a perfectly real green with none of your work
in it. This happened on GTW-944 (2026-08-01): a land agent got \`EXIT_TEST=0\`, "2584 passed", a
true green OF THE PREVIOUS TICKET, and only caught it because the count disagreed with the
forwarded evidence (2605) and \`git status\` came back clean when the tree should have been dirty.

So, without exception:
- Put \`cd <dir> && \` in front of EVERY cargo call, in the SAME call. Never rely on a previous one.
- Before you trust ANY suite result, run \`git -C <dir> status --short\` in the same call and
  confirm the modified/untracked files you expect are actually there. A clean tree when you have
  uncommitted work means you measured the wrong directory — discard the result and re-run.
- Cross-check the test COUNT against the count in any evidence you were forwarded. A count that
  moved when you changed nothing, or did not move when you added tests, means the wrong tree.
- Report which directory each result came from. "The suite is green" without a directory is not
  evidence.

The last two are PACKAGE-SCOPED aliases (\`-p gdtf_qa_protocol --features schema\`) added by
GTW-939. They are not optional and they are not covered by the workspace runs: \`schemars\` is an
optional dependency behind \`gdtf_qa_protocol\`'s \`schema\` feature, so every
\`#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]\` line and the whole
\`src/ids/test/schema.rs\` module is dark without them. Do NOT "simplify" them into a
\`--workspace\` run with \`gdtf_qa_protocol/schema\` named — that links schemars into
\`bins/gdtf_qa_mcp\`, and \`crates/gdtf_qa_protocol/tests/engine_free.rs\` fails if you try.

A sub-second "Finished" is NOT proof of a green run — \`.cargo/config.toml\` sets
\`checksum-freshness = true\`, so \`touch\` does not invalidate a cached result. Trust the exit code.
If you need to prove a lint actually covers a file, inject a real violation, confirm it fails,
then revert and confirm the revert is clean.
`

phase('Open')

const ticketText = await agent(`Read-only Linear work, then ONE status change.

1. Fetch ${TICKET} from the GDTF project: its FULL description AND its complete comment thread, verbatim. Comments routinely carry corrections that contradict the description — I need both.
2. Move ${TICKET} to **In Progress**. Do this NOW, at the start of the work, not at the end. Twelve tickets on this board went Backlog straight to Done because the status step only ever ran at the end, and Linear cannot backdate a transition.
3. Report the description and every comment verbatim. Do not summarise, do not paraphrase, do not "clean up" the acceptance criteria — the exact wording is the contract and a paraphrase is how this epic already shipped one ticket built to the wrong spec.

Also report: the ticket's current labels, its parent, and any blocking relationships it declares.`,
  { model: 'opus', label: `open:${TICKET}`, phase: 'Open', agentType: 'project-manager' })

if (!ticketText) throw new Error(`could not fetch ${TICKET}`)

phase('Build')

const built = await agent(`Implement ${TICKET} in a git worktree.

## Set up — RESUMABLE. A previous run may have been killed mid-build; its work is still on disk.

FIRST check whether the worktree already exists:
\`\`\`
git -C ${REPO} worktree list
\`\`\`

- **If ${WT} is NOT listed**, create it:
  \`\`\`
  git -C ${REPO} worktree add -b ${BRANCH} ${WT} develop
  \`\`\`
- **If ${WT} IS listed**, DO NOT try to create it — \`worktree add -b\` fails when the branch or
  directory already exists. Adopt it instead: run \`git -C ${WT} status --short\` and
  \`git -C ${WT} log --oneline -3\`, READ the changed files, and work out how much of the contract is
  already built. Continue from there. **Never delete or reset that work** — a killed run leaves
  finished, unlanded code, and rebuilding it from scratch wastes hours and risks losing it.
  Say in your report that you adopted an existing worktree and what state you found it in.

Work ONLY in ${WT}. Never touch ${REPO}'s working tree.

## THE CONTRACT — the ticket, verbatim. Build exactly this.

<ticket id="${TICKET}">
${ticketText}
</ticket>

Build every clause. Do not narrow, simplify, defer, or substitute any part of it. If a clause is
genuinely ambiguous, implement the reading that delivers MORE capability and say which reading you
took and why. If a clause turns out to be impossible or wrong, STOP and report — do not quietly
build something adjacent.
${NOTES}

## BEFORE YOU WRITE ANY CODE — the bootstrap check

Walk EVERY clause and ask: **does this clause demand evidence produced through a capability that
this very ticket is adding?** The classic shape is a clause requiring a pasted
\`mcp__gdtf-qa__*\` request/response for a TOOL THIS TICKET INTRODUCES.

Be precise about WHY such a clause fails, because the reason decides the fix. \`.mcp.json\` spawns
the courier as \`sh -c "cargo build -p gdtf_qa_mcp && exec target/debug/gdtf_qa_mcp"\` with
RELATIVE paths, so the connected courier is always built from the MAIN checkout on \`develop\` —
never from your branch. Two consequences, and only the second is fatal:

- **A new game/editor COMMAND is not automatically a blocker.** \`mcp__gdtf-qa__launch\` takes a
  \`working_dir\`, the router forwards \`Catalogue\` and \`Run\` generically, and the courier
  renders a PNG attachment with no per-command edit. A game launched from YOUR worktree can
  therefore publish your new command and answer a \`run\` for it.
- **Anything requiring a changed COURIER, or a changed protocol VERSION, genuinely cannot be
  proven live.** The courier is develop's build, so a courier-side change is not in it. And a
  version bump is worse than it looks: the courier sends develop's \`ProtocolVersion::CURRENT\`,
  your branch's app answers with the bumped one, the listener returns \`VersionMismatch\`, and
  then EVERY live call fails — including ones that have nothing to do with your ticket.

So: if a clause needs live evidence AND this ticket bumps the protocol version or edits the
courier, that clause is UNSATISFIABLE and no effort will fix it. Reconnecting does not help.
Killing the MCP process does not help — it removes every tool from the session until the user runs
\`/mcp\`.

**If you find such a clause: STOP IMMEDIATELY and report it. Do not start building.** Name the
clause, quote it, and say what evidence it demands and why it cannot exist yet. The orchestrator
will split the live proof into a follow-up ticket blocked on this one.

This has now happened THREE times in this epic — GTW-879 clause 7, GTW-875 clauses 2-3, GTW-808
clauses 1/3/4/5/6. The GTW-808 case burned three full gate rounds and a fix agent before anyone
noticed. Catch it here, at the front, where it costs one message.

${HOUSE_RULES}

${GREEN}

Run the suite yourself before reporting. If it is red, fix it and run again. Do NOT commit —
committing is a later step with a different agent.

Report: every file you changed and why, the verbatim result of each suite step, and — clause by
clause, numbered to match the ticket — what you did and where the evidence for it is.`,
  { model: 'opus', label: `build:${TICKET}`, phase: 'Build' })

if (!built) throw new Error(`build agent died on ${TICKET}`)

// Re-fetch the ticket before verifying. A ticket can be CORRECTED while its build runs — an
// unsatisfiable clause found mid-flight, a scope fix, a user comment — and verifying or gating
// against the text captured at Open time audits the work against a specification that no longer
// exists. That is the GTW-864 failure (reviewers passing work against the wrong contract) arriving
// from a different direction.
const fresh = await agent(`Re-fetch ${TICKET} from Linear. READ-ONLY — change nothing, move no status.

Return its CURRENT full description and its COMPLETE comment thread, verbatim. Do not summarise or
tidy the acceptance criteria; the exact wording is a contract.

Context: this ticket may have been amended since its build started. If you can tell what changed,
say so explicitly — particularly any acceptance clause that was rewritten, renumbered, added, or
marked deferred to another ticket.`,
  { model: 'opus', label: `refetch:${TICKET}`, phase: 'Verify', agentType: 'project-manager' })

const contract = fresh || ticketText
if (fresh && fresh !== ticketText) {
  log(`${TICKET}: contract re-fetched — verifying against the CURRENT ticket text, not the text captured at start`)
}

async function verify(attempt) {
  return agent(`Independently verify ${TICKET} in the worktree ${WT}. Trust NOTHING the implementer reported — re-run everything yourself.

**This is the ticket's CURRENT text, re-fetched just now.** If it differs from what the implementer
built against, the current text wins — and say plainly which clauses moved.

<ticket id="${TICKET}">
${contract}
</ticket>

<implementer-report attempt="${attempt}">
${built}
</implementer-report>

${GREEN}

## Then verify the ticket's OWN evidence clauses

The acceptance criteria demand specific evidence — a launch command, a log line, a port check, an
MCP tool call. Produce each one yourself and paste the real output. An evidence clause you did not
personally reproduce is NOT satisfied, no matter what the implementer claimed.

${HOUSE_RULES}

Start your answer with GREEN or RED on its own line. GREEN means every suite step exited 0 AND
every evidence clause was reproduced by you. Anything else is RED. If RED, quote the exact failing
output and name the clause it breaks.`,
    { model: 'opus', label: `verify:${TICKET}#${attempt}`, phase: 'Verify' })
}

phase('Verify')
let verifyOut = await verify(1)

const readVerdict = (out) => {
  if (!out) return false
  const m = /\b(GREEN|RED)\b/i.exec(out)
  return !!m && m[1].toUpperCase() === 'GREEN'
}

const LENSES = [
  { key: 'fidelity', focus: `Does the diff implement EVERY clause of the ticket, exactly as written? Hunt for silent narrowing — a clause half-built, a hard case skipped, a "for now" shortcut. Check the diff against docs/ as well: docs/ is the design contract alongside the ticket. Flag anything built that the ticket did NOT ask for.` },
  { key: 'tests', focus: `Do the tests exercise the REAL code path? Reject any test that builds a stand-in app and asserts against hand-inserted state where the real app would produce it. This epic already shipped tests on MinimalPlugins that assert values false in the real editor — that pattern must not land again. For every new behaviour, name the mutation that would slip past the tests. Take the suite result in the verification evidence below as given; run ZERO cargo yourself.` },
  { key: 'structure', focus: `Rust and Bevy structure: module layout (a module is a directory, mod.rs is wiring-only with no fns, warn over 300 lines and block over 400), no bare primitives for domain values, no unwrap/expect/panic/todo/unimplemented, docs on every pub item. Bevy specifics: correct schedule placement, explicit system ordering, no exclusive &mut World where SystemParams would do, state-scoped resources handled correctly. Run ZERO cargo.` },
]

async function gate(attempt) {
  const verdicts = await parallel(LENSES.map(l => () =>
    agent(`Adversarial read-only review of ${TICKET} in the worktree ${WT}. You are a gate — your job is to find what is wrong, not to approve.

Audit against the ticket text below — it is the ticket's CURRENT wording, re-fetched after the
build ran, so it may differ from what the implementer worked to. Judge the work against THIS.
If a clause is explicitly marked deferred to another ticket, that deferral is the contract: do NOT
block on evidence the ticket itself defers.

<ticket id="${TICKET}">
${contract}
</ticket>

## YOUR LENS
${l.focus}

## VERIFICATION EVIDENCE — the suite run you are told to take as given
<verify-report>
${verifyOut || '(THE VERIFY AGENT DIED — no suite evidence exists. Treat every cargo-dependent claim as UNPROVEN and say so.)'}
</verify-report>

${HOUSE_RULES}

**Run ZERO cargo commands** — other agents may be building, and concurrent cargo corrupts this
repo's shared build artifacts. Read the diff (\`git -C ${WT} diff develop\`) and the files.

Cite file:line for every finding. A finding you cannot cite is not a finding.

Start your answer with COMPLIANT or NON-COMPLIANT on its own line. NON-COMPLIANT if ANY clause is
unmet or any standing rule is broken. If NON-COMPLIANT, list each violation with its file:line and
exactly what must change.`,
      { model: 'opus', label: `gate:${l.key}#${attempt}`, phase: 'Gate', agentType: 'design-gate' })
  ))
  return verdicts
}

phase('Gate')
let attempt = 1
let verdicts = await gate(attempt)

const compliant = (out) => {
  if (!out) return false
  const m = /\b(NON-COMPLIANT|COMPLIANT)\b/i.exec(out)
  return !!m && m[1].toUpperCase() === 'COMPLIANT'
}

const allPass = () => readVerdict(verifyOut) && verdicts.every(compliant)

// A DEAD reviewer is not the same thing as a reviewer that found violations, and the difference
// decides whether a fix agent should run at all. Failing closed on a dead lens is right — silence
// must never read as a pass. But if NOTHING reported, there are no findings to repair, and handing
// a fix agent a prompt whose only content is died-placeholders sets it loose on code no reviewer
// has actually faulted. It then either invents work or "fixes" what was already correct, and on a
// green tree that is pure damage. This happened on GTW-923 (2026-07-30): a 529 storm killed the
// refetch, the verify and all three lenses, the loop entered round 2, and only the fix agent dying
// too kept it away from a tree whose suite was green (2538 tests, 207 targets, six steps exit 0).
// So: distinguish "no verdict obtained" from "verdict was negative". No verdict at all is an
// INFRASTRUCTURE failure — abort and let the orchestrator resume, which replays the cached build
// and costs nothing. Any real finding, even one, still enters the repair loop as before.
const reviewersHeardFrom = () => (verifyOut ? 1 : 0) + verdicts.filter(Boolean).length

while (!allPass() && attempt < 5) {
  if (reviewersHeardFrom() === 0) {
    throw new Error(
      `${TICKET}: no reviewer reported — verify and all ${verdicts.length} gate lenses died ` +
      `(round ${attempt}). This is an infrastructure failure, NOT a gate finding: there is ` +
      `nothing to repair, so no fix agent is being launched against the worktree. The work in ` +
      `${WT} is intact and uncommitted. RESUME this run — the build replays from cache.`,
    )
  }
  attempt++
  phase('Fix')
  const problems = [
    readVerdict(verifyOut) ? null : `<verify-verdict>\n${verifyOut || '(verify agent died — no evidence)'}\n</verify-verdict>`,
    ...verdicts.map((v, i) => compliant(v) ? null : `<gate-lens name="${LENSES[i].key}">\n${v || '(lens died — treat as blocking)'}\n</gate-lens>`),
  ].filter(Boolean).join('\n\n')

  log(`${TICKET}: round ${attempt} — repairing blocking findings`)

  const fixed = await agent(`Repair ${TICKET} in the worktree ${WT}. The verification and/or the gate blocked it.

This is the ticket's CURRENT text, re-fetched after the build ran. If it differs from what was
originally built against, this wins.

<ticket id="${TICKET}">
${contract}
</ticket>

## WHAT BLOCKED IT
${problems}
${NOTES}

Fix every blocking finding at its root. Do NOT weaken a test, loosen an assertion, delete a check,
or edit a gate to make a complaint go away — the reviewers are doing their job and the fix belongs
in the code. If you believe a finding is simply wrong, say so with cited evidence rather than
silently ignoring it.

${HOUSE_RULES}

${GREEN}

Re-run the full suite yourself before reporting. Do NOT commit.

Report what you changed for each finding, and the verbatim suite result.`,
    { model: 'opus', label: `fix:${TICKET}#${attempt}`, phase: 'Fix' })

  if (!fixed) throw new Error(`fix agent died on ${TICKET} round ${attempt}`)

  phase('Verify')
  verifyOut = await verify(attempt)
  phase('Gate')
  verdicts = await gate(attempt)
}

if (!allPass()) {
  return {
    ticket: TICKET,
    landed: false,
    reason: `still blocked after ${attempt} rounds`,
    verify: verifyOut,
    gate: verdicts,
  }
}

phase('Land')

const landed = await agent(`Land ${TICKET} — but only after satisfying yourself it deserves to land.

Do NOT take a pass on faith. Below is the actual evidence, forwarded in full rather than
summarised as a claim, so you can judge it yourself. Re-run the suite regardless (step 1). If any
of it looks wrong, or the evidence does not support landing, STOP and report — a blocked land is a
result, never an obstacle to route around. Never use \`--no-verify\`.

## VERIFICATION EVIDENCE — the suite run, verbatim
<verify-report>
${verifyOut}
</verify-report>

## GATE VERDICTS — all three lenses, verbatim, from the final round (${attempt} round(s) run)
${verdicts.map((v, i) => `<gate-lens name="${LENSES[i].key}">\n${v}\n</gate-lens>`).join('\n')}

If this ticket needed more than one round, earlier rounds DID block and were repaired — the
verdicts above are the final state, not the only state. Say so in your report rather than implying
it passed first time.

- Worktree: ${WT}
- Branch: ${BRANCH}
- Main repo: ${REPO} (on develop)

## Steps
1. Rebase onto current develop. If develop moved, **re-run the full green suite** — the gate result is stale otherwise.
   ${GREEN}
2. Write \`${WT}/.claude/.gate-pass\` (TICKET / BRANCH / HEAD / FINGERPRINT). Compute the fingerprint, then RECOMPUTE it and confirm an exact match before trusting it. It is gitignored; never stage it.
3. Stage files EXPLICITLY by name. Never \`git add -A\`. Check for untracked files and stage each deliberately, then verify the INDEX with \`git diff --cached --name-only\` — the index is what you are about to commit, so confirm it directly rather than inferring from \`git status\`.
4. Commit with \`git -C ${WT} commit\` (NOT \`cd\` then commit). Subject: \`Area: summary (${TICKET})\`, matching the voice of \`git log --oneline -15\`. Body says what changed and why. No banned jargon.

   **If you write the message to a file, the filename MUST contain ${TICKET}** — e.g.
   \`${TICKET}-commit-message.txt\`. NEVER a generic name like \`msg.txt\`. The scratchpad is shared
   across every land this session and generic names survive; a previous ticket's message file is
   still sitting there right now.

   **Write the message file in its own command, THEN commit in a separate command.** Do not
   combine the two: if the pre-commit hook blocks a combined call, the message file is never
   written, and the retry silently picks up whatever stale file already exists.

   **After committing, read the subject back with \`git -C ${WT} log -1 --format=%s\` and confirm it
   names ${TICKET}.** This exact failure happened on GTW-882 — a blocked combined call, then a
   retry that reused GTW-808's leftover \`msg.txt\`, producing a commit titled for the wrong
   ticket. It was caught only by reading the subject back. If it is wrong, fix it with
   \`git -C ${WT} commit --amend -F <the ${TICKET}-named file>\` and say so in your report.
5. Run \`git flow feature finish\` from **${REPO}**, not the worktree. Expected: it fails to delete the branch while the worktree holds it, and leaves \`.git/gitflow/state/merge.json\`, which falsely blocks the NEXT land with "a merge is already in progress" (GTW-874).
6. Push develop. Confirm the commit actually reached origin/develop with \`git ls-remote\`.
7. Clean up: \`git worktree remove ${WT}\`, \`git branch -d ${BRANCH}\`, then remove the stale \`merge.json\` — ONLY after confirming no real merge is in flight. The worktree holds tens of GB; removing it matters.

If the pre-commit hook blocks you for any reason other than a stale gate-pass, do NOT use
\`--no-verify\`. STOP and report.

Report the commit SHA, the files, the push result and the cleanup outcome, quoting what git
actually said.`,
  { model: 'opus', label: `land:${TICKET}`, phase: 'Land' })

const landedOk = !!landed && /[0-9a-f]{7,40}/.test(landed)

if (landedOk) {
  await agent(`Close ${TICKET} in Linear with evidence.

Move it to **In Review** first, then **Done** — it is currently In Progress and the board should show the path the work actually took.

Post a comment BEFORE changing status (archived issues reject comments, and there is no un-archive) recording:
- The landing commit and confirmation it reached origin/develop.
- The full green suite result.
- The ticket's own evidence clauses and how each was satisfied — quote the actual output, not a claim that it passed.
- Any judgement call the implementer made, and anything deliberately left out of scope.

Here is the landing report:
<land-report>
${landed}
</land-report>

And the verification evidence:
<verify-report>
${verifyOut}
</verify-report>

Then check ${TICKET} for sub-issues BEFORE and AFTER the close — marking a parent Done on this
board has silently auto-completed open children. Report anything that moved.

Plain language, no banned jargon.`,
    { model: 'opus', label: `close:${TICKET}`, phase: 'Land', agentType: 'project-manager' })
}

return { ticket: TICKET, landed: landedOk, rounds: attempt, land: landed, verify: verifyOut }

export const meta = {
  name: 'build-ticket',
  description: 'Build one GTW ticket end to end: clause-audit, open, build, verify, 3-lens gate, docs-sync, land, close. Uses the skills and the single green definition in .claude/rules/verification.md.',
  phases: [
    { title: 'Clause-audit', detail: 'phase 0: audit live ticket text before In Progress (GTW-962)' },
    { title: 'Open', detail: 'mark ticket In Progress after a clean audit' },
    { title: 'Build', detail: 'implement against the quoted contract' },
    { title: 'Verify', detail: 'full green suite (verification.md) plus the ticket evidence clauses' },
    { title: 'Gate', detail: '3 read-only lenses, any-non-compliant blocks (/gate skill)' },
    { title: 'Fix', detail: 'bounded repair loop on a red verdict' },
    { title: 'Docs-sync', detail: 're-align docs/ if the change drifted design claims (/docs-sync skill)' },
    { title: 'Land', detail: 'commit, rebase onto develop, fast-forward, push, close (/land skill)' },
  ],
}

// Green suite: ALWAYS the aliases from .claude/rules/verification.md.
// Skills /gate, /docs-sync, /land are the source of truth for process.
//
// Every agent answers through a schema. Nothing here reads a verdict out of prose —
// that cost repair rounds on correct code when a report opened "NON-COMPLIANT? No — COMPLIANT".

// args: { ticket: "GTW-123", slug: "gtw-123-some-slug" }
// Accept either a real object or a JSON-encoded string — the harness may deliver either.
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { throw new Error(`args was an unparseable string: ${A}`) }
}
const TICKET = A?.ticket
const SLUG = A?.slug
if (!TICKET || !SLUG) throw new Error(`args must supply { ticket, slug }; got ${JSON.stringify(args)}`)

// Repo root: workflow cwd, not a hard-coded home path (GTW-954).
const REPO = (typeof process !== 'undefined' && process.cwd && process.cwd()) || '.'
const BRANCH = `feature/${SLUG}`

// --- Schemas ---------------------------------------------------------------

const TICKET_TEXT = {
  type: 'object', additionalProperties: false,
  required: ['text'],
  properties: {
    text: { type: 'string', description: 'Full description and every comment, verbatim, plus labels, parent and blocking relationships.' },
  },
}

const AUDIT_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'report'],
  properties: {
    verdict: { type: 'string', enum: ['AUDIT_OK', 'AUDIT_BLOCK'], description: 'AUDIT_BLOCK only for a product decision the code cannot answer.' },
    report: { type: 'string', description: 'Every word of the audit, with the CORRECTIONS section in full. The builder builds from this.' },
  },
}

const WORK_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['filesChanged', 'suite', 'report'],
  properties: {
    filesChanged: { type: 'array', items: { type: 'string' }, description: 'Every path you created or edited, by name, exactly as git reports it. Empty only if you truly changed nothing.' },
    suite: { type: 'array', description: 'One row per suite command, with the exit code you read yourself.',
      items: { type: 'object', additionalProperties: false, required: ['command', 'exit'],
        properties: { command: { type: 'string' }, exit: { type: 'integer' } } } },
    report: { type: 'string', description: 'Clause-by-clause evidence. Failures quoted whole.' },
  },
}

const VERIFY_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'report', 'reportMatchesTree'],
  properties: {
    verdict: { type: 'string', enum: ['GREEN', 'RED'], description: 'GREEN only when every suite command exited 0 after the final edit AND every evidence clause reproduced.' },
    reportMatchesTree: { type: 'boolean', description: 'Did the implementer\'s filesChanged list match `git status --porcelain`? False if it named files it did not touch, or missed files it did.' },
    report: { type: 'string', description: 'Every word of the verification: the suite table, each clause reproduced, failures quoted whole.' },
  },
}

const LENS_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'report'],
  properties: {
    verdict: { type: 'string', enum: ['COMPLIANT', 'NON-COMPLIANT'], description: 'NON-COMPLIANT only for a clause you can show is unmet.' },
    report: { type: 'string', description: 'Every word of the review: per-clause findings, each naming the symbol and quoting the line. Write it for the agent that has to repair the code.' },
  },
}

const DOCS_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['drifted', 'filesChanged', 'report'],
  properties: {
    drifted: { type: 'boolean', description: 'Did any doc claim disagree with the code' },
    filesChanged: { type: 'array', items: { type: 'string' }, description: 'Docs you edited, by name. Empty when nothing drifted.' },
    report: { type: 'string' },
  },
}

// The land agent holds no Linear tools, so it has no field to guess with. Landing is
// proven by the confirm step and a first-hand git check, never by this report.
const LAND_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['committed', 'commitSha', 'pushedRange', 'filesStaged', 'suite', 'findings'],
  properties: {
    committed: { type: 'boolean', description: 'Did the commit and push actually run' },
    commitSha: { type: 'string', description: 'Feature commit sha, or empty if none' },
    pushedRange: { type: 'string', description: 'Exactly what git push printed, or empty' },
    filesStaged: { type: 'array', items: { type: 'string' }, description: 'Every path staged, by name. Never a glob or a count.' },
    suite: { type: 'array', description: 'One row per suite command, with the exit code you read yourself.',
      items: { type: 'object', additionalProperties: false, required: ['command', 'exit'],
        properties: { command: { type: 'string' }, exit: { type: 'integer' } } } },
    findings: { type: 'array', items: { type: 'string' }, description: 'Anything outside this ticket worth a look. Not ticket status — you cannot see the board.' },
  },
}

const CONFIRM_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['landedSha'],
  properties: {
    landedSha: { type: 'string', description: 'The sha on origin/develop whose subject names this ticket, or the empty string if there is none.' },
  },
}

// The last agent in the run, and the only one that can see the board. Whatever the
// orchestrator needs to report to the user comes from here — anything left out of this
// schema is discarded when the run ends.
const CLOSE_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['closed', 'ticketState', 'summary', 'boardEffects', 'report'],
  properties: {
    closed: { type: 'boolean', description: 'Is the ticket Done on the board now' },
    ticketState: { type: 'string', description: 'The state name the board shows for this ticket now, whatever it is.' },
    summary: { type: 'string', description: 'Two or three plain sentences a person can read without opening anything: what shipped and what it changes. Not a list of files. plain-language.md applies.' },
    boardEffects: {
      type: 'array', items: { type: 'string' },
      description: 'Anything the close changed beyond this ticket — a child auto-completed, a parent auto-cancelled, a relation that vanished. Empty if none. Check children before and after.',
    },
    report: { type: 'string', description: 'What you posted and what the board said back.' },
  },
}

// --- Prompt fragments ------------------------------------------------------

const HOUSE_RULES = `
## Standing rules — not negotiable

1. Drive the app ONLY through the \`mcp__gdtf-qa__*\` tools. Never a script, never a socket.
   A missing tool is a STOP-and-report, not a reason to rebuild or kill anything by hand.
2. Cheapest sufficient evidence: integration test ≈ unit test > reading the code >> building tooling.
3. Prove behaviour against the REAL binary. MinimalPlugins with hand-inserted resources proves nothing.
4. No unwrap/expect/panic/todo/unimplemented. Doc every pub item. Typed domain values (no-bare-types.md).
   Files: warn >300, block >400. mod.rs is wiring only.
5. Comments: comment-hygiene.md. Symbols and bulk edits: code-navigation.md — the LSP answers every
   symbol question, and no script ever edits Rust source.
6. plain-language.md governs everything you write. Quote failures whole; never summarise one away.
7. Only the project-manager steps can write to Linear. "Did not file a ticket" is never a valid
   finding against a build. Put anything ticket-worthy in your report under an out-of-scope heading;
   the orchestrator files it.
8. A designed behaviour is not a gap. Before reporting that two numbers disagree, grep the tests for
   an assertion that they disagree on purpose.
`

const GREEN = `
## Green

READ \`${REPO}/.claude/rules/verification.md\` AND RUN THE SUITE IT LISTS. That file is the only
authority; a copy here would go stale. Use the \`.cargo/config.toml\` aliases exactly as written.

Run them SEQUENTIALLY, one command per tool call, and read each exit code on its own. Pin the
directory on every cargo call (\`cd ${REPO} && ...\`) — the Bash tool cwd resets between calls.

NEVER pipe cargo into \`tail\`/\`head\`/\`grep\` inside an \`&&\` chain. The pipeline's exit code is
the filter's, which always succeeds, so a failing step reports green. That produced a false
all-green while \`cargo doc\` was exiting 101.

Green means every command in that file exited 0, after your final edit.
`

// --- Phase 0: clause audit (GTW-962) — BEFORE In Progress ---
// Never audit a summary argument. Only live Linear description + comments.
phase('Clause-audit')

const live = await agent(`Read-only Linear fetch for ${TICKET} in project GDTF.

Fetch the FULL description AND the complete comment thread. Do NOT change status.
Return them verbatim in \`text\`, together with labels, parent and blocking relationships.`,
  { model: 'opus', label: `fetch:${TICKET}`, phase: 'Clause-audit', agentType: 'project-manager', schema: TICKET_TEXT })

if (!live) throw new Error(`could not fetch ${TICKET} for clause audit`)

const audit = await agent(`Clause audit for ${TICKET}.

<live-ticket id="${TICKET}">
${live.text}
</live-ticket>`,
  { model: 'opus', label: `clause-audit:${TICKET}`, phase: 'Clause-audit', agentType: 'clause-audit', schema: AUDIT_RESULT })

if (!audit) throw new Error(`${TICKET}: clause audit died — abort`)

log(`${TICKET}: clause audit ${audit.verdict}`)

if (audit.verdict !== 'AUDIT_OK') {
  return { ticket: TICKET, landed: false, reason: 'clause audit blocked before In Progress', audit: audit.report }
}

phase('Open')

// The audit's corrections are what the builder builds to, and they live nowhere but this run.
// Post them so a reader of the ticket can see what was actually built against, and so a run that
// dies after the audit does not take them with it.
const opened = await agent(`Linear status and one comment for ${TICKET}.

1. Post the comment below FIRST, before any status change.
2. Move ${TICKET} to **In Progress**.
3. Re-fetch the FULL description and complete comment thread (the status change may race with board edits).
4. Return them verbatim in \`text\`, with labels, parent and blocking relationships.

The comment is the clause audit's corrections, posted verbatim under a source line so nobody reads
them as the owner's. Per \`.claude/rules/linear-discipline.md\` it opens with the source line and
nothing above it. Post exactly this, changing nothing inside it:

---
**[clause-audit]**

Corrections applied to this ticket's clauses for the build starting now. These are the audit's
readings, not a ruling by the owner. The build was made against them wherever they differ from the
description above.

${audit.report}
---`,
  { model: 'opus', label: `open:${TICKET}`, phase: 'Open', agentType: 'project-manager', schema: TICKET_TEXT })

if (!opened) throw new Error(`could not open ${TICKET}`)

phase('Build')

const built = await agent(`Implement ${TICKET} in the MAIN repo at ${REPO}.

## Set up — RESUMABLE

All work happens in ${REPO}, on ${BRANCH}. Never commit on develop; the pre-commit hook blocks it.

Check what is already checked out (\`git -C ${REPO} branch --show-current\`, \`git -C ${REPO} status --short\`):

- ${BRANCH} already checked out: adopt it. Never reset or discard unlanded work.
- Exists but not checked out: \`git -C ${REPO} checkout ${BRANCH}\`
- Otherwise, from a clean tree on develop: \`git -C ${REPO} checkout develop && git -C ${REPO} checkout -b ${BRANCH}\`

If the tree is dirty with work that is not this ticket's, leave those files exactly as they are —
do not stage them, do not revert them, and do not build on top of them. Name them in your report.

## THE CONTRACT — build exactly this

<ticket id="${TICKET}">
${opened.text}
</ticket>

## CLAUSE AUDIT — the CORRECTIONS section is BINDING

The audit opened every file the ticket cites and checked it against the tree. Where a clause was
wrong it wrote a corrected one. **Build the corrected clauses** wherever the two differ; everything
it did not correct stands as the ticket wrote it.

${audit.report}

Build every clause. Do not narrow. If a clause is impossible, STOP and report.

If a clause demands live evidence for a capability THIS ticket is adding — a change to the MCP
server (\`gdtf_qa_mcp\`) or a protocol version bump — that clause cannot be met. STOP and report.

${HOUSE_RULES}
${GREEN}

Run the suite yourself before reporting. Do NOT commit.

\`filesChanged\` must match \`git status --porcelain\` exactly. Verify checks it against the tree.`,
  { model: 'opus', label: `build:${TICKET}`, phase: 'Build', agentType: 'engineer', schema: WORK_RESULT })

if (!built) throw new Error(`build agent died on ${TICKET}`)

// The contract everyone downstream judges against: the live ticket text PLUS the audit's
// corrections. The corrections are never written back to Linear, so a re-fetch alone loses them —
// and then the builder builds to one contract while verify and the lenses judge another. On
// GTW-1012 that cost five gate rounds and a non-landing. The audit goes LAST so it wins on conflict.
const fresh = await agent(`Re-fetch ${TICKET} from Linear. READ-ONLY. Return the CURRENT full
description and complete comment thread, verbatim, in \`text\`.`,
  { model: 'opus', label: `refetch:${TICKET}`, phase: 'Verify', agentType: 'project-manager', schema: TICKET_TEXT })

const contract = `${fresh?.text || opened.text}

## CLAUSE AUDIT (this run) — the CORRECTIONS section is BINDING and OVERRIDES the text above

The corrections were NOT written back to Linear, so the text above is the uncorrected original.
Where they differ, the corrections win — the implementer built to them. Do NOT report a violation
for failing to do something a correction struck out, nor for doing what a correction requires.

${audit.report}`

// --- Verify and gate -------------------------------------------------------

// One lens per question, and each owns its own failure modes. They used to share a checklist in
// the agent definition, which is how three reviews came back saying the same thing.
const LENSES = [
  { key: 'clauses', focus: `Is every clause true of the code? Open each one and trace it — "the report says so" is not evidence.
Hunt quiet narrowing: a clause half-built reads as built. Reject hedge markers — TODO, FIXME, "for now",
"placeholder", "stub", "simplified". Reject any system, plugin or resource the ticket claims runs that
nothing registers. Check docs/ against the same clauses.` },
  { key: 'tests', focus: `Is the behaviour actually proven? Every behavioural clause needs a real-path,
assertion-bearing test that discriminates — name the mutation that would slip past each one. A clause with
no test is NON-COMPLIANT, not a note. Reject MinimalPlugins stand-ins where the claim is about the real app,
and reject exact-magnitude asserts on tunable data. Run ZERO cargo.` },
  { key: 'rules', focus: `Does it obey the house? no-bare-types, module-layout (including files over 400 lines
with mixed responsibilities), bevy-systems scheduling and ordering, plain-language, comment-hygiene.
Cite the rule you are applying, never a preference. Run ZERO cargo.` },
]

async function verify(work, attempt) {
  return await agent(`Independently verify ${TICKET} in ${REPO}. Trust NOTHING the implementer reported.

<ticket id="${TICKET}">
${contract}
</ticket>

<implementer-report attempt="${attempt}">
${work.report}
</implementer-report>

<implementer-files-changed>
${work.filesChanged.join('\n') || '(claimed none)'}
</implementer-files-changed>

${GREEN}

Reproduce every evidence clause yourself.

Run \`git -C ${REPO} status --porcelain\` and compare it against the claimed file list above. Set
\`reportMatchesTree\` false if the implementer named a file it did not touch or missed one it did,
and say which in your report. An implementer has reported doing nothing while its branch held six
edited files, so this is a real check, not a formality.

${HOUSE_RULES}`,
    { model: 'opus', label: `verify:${TICKET}#${attempt}`, phase: 'Verify', schema: VERIFY_RESULT })
}

async function runLens(lens, verifyOut, attempt) {
  return await agent(`Adversarial read-only review of ${TICKET} in ${REPO}.

<ticket id="${TICKET}">
${contract}
</ticket>

## YOUR LENS
${lens.focus}

## VERIFICATION EVIDENCE
<verify-report verdict="${verifyOut?.verdict ?? 'MISSING'}">
${verifyOut?.report ?? '(verify died — treat cargo claims as UNPROVEN)'}
</verify-report>

${HOUSE_RULES}

Run ZERO cargo. Name the symbol and quote the line.

Say NON-COMPLIANT only for a clause you can show is unmet, and cite the line that shows it. Put
every word of your reasoning in \`report\` — the repair agent reads it, so a finding you leave out
is a finding nobody fixes.`,
    { model: 'opus', label: `gate:${lens.key}#${attempt}`, phase: 'Gate', agentType: 'design-gate', schema: LENS_RESULT })
}

phase('Verify')
let work = built
let verifyOut = await verify(work, 1)

phase('Gate')
let attempt = 1
let verdicts = await parallel(LENSES.map(l => () => runLens(l, verifyOut, attempt)))

const green = () => verifyOut?.verdict === 'GREEN'
const compliant = (v) => v?.verdict === 'COMPLIANT'
const allPass = () => green() && verdicts.every(compliant)
const reviewersHeardFrom = () => (verifyOut ? 1 : 0) + verdicts.filter(Boolean).length

while (!allPass() && attempt < 5) {
  if (reviewersHeardFrom() === 0) {
    throw new Error(`${TICKET}: no reviewer reported (round ${attempt}). Infrastructure failure — resume.`)
  }
  attempt++
  phase('Fix')

  const problems = [
    green() ? null : verifyOut
      ? `<verify-report verdict="${verifyOut.verdict}"${verifyOut.reportMatchesTree ? '' : ' files-claimed-do-not-match-the-tree="true"'}>\n${verifyOut.report}\n</verify-report>`
      : `<verify-report>\n(died — nothing to act on)\n</verify-report>`,
    ...verdicts.map((v, i) => {
      if (compliant(v)) return null
      if (!v) return `<gate-lens name="${LENSES[i].key}">\n(died — nothing to act on)\n</gate-lens>`
      return `<gate-lens name="${LENSES[i].key}" verdict="${v.verdict}">\n${v.report}\n</gate-lens>`
    }),
  ].filter(Boolean).join('\n\n')

  log(`${TICKET}: round ${attempt} — repairing`)

  work = await agent(`Repair ${TICKET} in ${REPO}.

<ticket id="${TICKET}">
${contract}
</ticket>

## WHAT BLOCKED IT
${problems}

Fix at root. Do NOT weaken tests and do NOT edit the gate.

Files dirty in the tree that are not this ticket's work stay exactly as they are — do not stage
them and do not revert them. A repair round has discarded an unrelated edit before.

${HOUSE_RULES}
${GREEN}

Re-run the suite. Do NOT commit. \`filesChanged\` must match \`git status --porcelain\`.`,
    { model: 'opus', label: `fix:${TICKET}#${attempt}`, phase: 'Fix', schema: WORK_RESULT })

  if (!work) throw new Error(`fix agent died on ${TICKET} round ${attempt}`)

  phase('Verify')
  verifyOut = await verify(work, attempt)
  phase('Gate')
  verdicts = await parallel(LENSES.map(l => () => runLens(l, verifyOut, attempt)))
}

if (!allPass()) {
  return {
    ticket: TICKET, landed: false, reason: `still blocked after ${attempt} rounds`,
    verify: verifyOut?.report ?? null,
    gate: verdicts.map((v, i) => ({ lens: LENSES[i].key, verdict: v?.verdict ?? 'MISSING', report: v?.report ?? null })),
  }
}

phase('Docs-sync')

await agent(`Docs-sync posture for ${TICKET} in ${REPO}.

If the change touched behaviour described in docs/, verify each claim against the code and fix the
drift. Code is authority for what exists; docs/ remain authority for design intent. If nothing
drifted, say so and stop.

Files dirty in the tree that are not this ticket's work stay exactly as they are.

Do NOT commit — /land owns the commit.`,
  { model: 'opus', label: `docs-sync:${TICKET}`, phase: 'Docs-sync', schema: DOCS_RESULT })

phase('Land')

const landed = await agent(`Land ${TICKET} following the /land skill.

Repo: ${REPO} — no worktree; the work is on ${BRANCH} in the main tree.

## VERIFICATION EVIDENCE
<verify-report verdict="${verifyOut.verdict}">
${verifyOut.report}
</verify-report>

## GATE VERDICTS (final round ${attempt})
${verdicts.map((v, i) => `<gate-lens name="${LENSES[i].key}" verdict="${v?.verdict ?? 'MISSING'}">\n${v?.report ?? '(died)'}\n</gate-lens>`).join('\n')}

Run every command in the foreground and read its exit code in the same turn. Never start the suite
in the background and end your turn waiting on it — that is how a previous attempt at this step died
without committing. There is no monitor coming to report your exit codes.

${GREEN}

Steps (see .claude/skills/land/SKILL.md):
1. Re-run the full green suite. Docs-sync runs after the gate, so the tree you are about to commit
   is not the tree the verify report certified. The build cache makes it cheap when nothing changed.
2. Write .claude/.gate-pass (TICKET/BRANCH/HEAD/FINGERPRINT/SCOPE). FINGERPRINT is the hex from the
   one command in .claude/rules/verification.md — do not invent a recipe.
3. Stage explicit files by name. Never git add -A. Files dirty in the tree that are not this
   ticket's work are left alone — not staged, not reverted.
4. Commit: \`Area: summary (${TICKET})\`. The body ends the message — no session URL, no
   Co-Authored-By, nothing after it.
5. From ${REPO}: fetch origin develop, rebase ${BRANCH} onto origin/develop, checkout develop, pull,
   \`git merge --ff-only ${BRANCH}\`, push origin develop, delete the local feature branch. There is
   no merge commit. If --ff-only fails, STOP and report — never fall back to a merge commit.
6. Leave ${REPO} on develop.

If the rebase conflicts, resolve it, stage the resolved files, and continue with
\`git rebase --continue\` — never the commit subcommand. The pre-commit hook blocks every commit on
develop.

Never --no-verify. Report what git said. Do not claim landing is proven — a separate confirm step
checks origin/develop.`,
  { model: 'opus', label: `land:${TICKET}`, phase: 'Land', schema: LAND_RESULT })

const confirm = await agent(`Confirm landing of ${TICKET}.

Main repo: ${REPO}

1. \`git -C ${REPO} fetch origin develop\`
2. Find the commit on origin/develop whose subject contains ${TICKET} (most recent if several).
3. \`git -C ${REPO} merge-base --is-ancestor <sha> origin/develop\`

Return that sha in \`landedSha\` only if step 3 exits 0. Otherwise return the empty string.
Check nothing else and change nothing.`,
  { model: 'opus', label: `confirm-land:${TICKET}`, phase: 'Land', schema: CONFIRM_RESULT })

const landedSha = /^[0-9a-f]{7,40}$/i.test(confirm?.landedSha ?? '') ? confirm.landedSha : null

const closed = landedSha
  ? await agent(`Close ${TICKET} in Linear with evidence.

Move to In Review then Done. Comment BEFORE the status change — an archived issue rejects a
comment and there is no un-archive. The comment opens with the source line \`**[build-ticket / land]**\`
and nothing above it, per \`.claude/rules/linear-discipline.md\`, then carries: landing sha ${landedSha},
the suite result, and which evidence clauses were satisfied.

Check the ticket's children before AND after the close. Cascades run both ways and are not
reliable, so report in \`boardEffects\` anything that moved besides this ticket.

This is the last step of the run. Your \`summary\` is what the orchestrator reports to the user,
so write it for someone who has read none of the below.

<land-result>
${JSON.stringify(landed, null, 2)}
</land-result>

<verify-report verdict="${verifyOut.verdict}">
${verifyOut.report}
</verify-report>

<gate-verdicts rounds="${attempt}">
${verdicts.map((v, i) => `<gate-lens name="${LENSES[i].key}" verdict="${v?.verdict ?? 'MISSING'}"/>`).join('\n')}
</gate-verdicts>`,
    { model: 'opus', label: `close:${TICKET}`, phase: 'Land', agentType: 'project-manager', schema: CLOSE_RESULT })
  : null

return {
  ticket: TICKET,
  landed: !!landedSha,
  landedSha,
  rounds: attempt,
  summary: closed?.summary ?? null,
  ticketState: closed?.ticketState ?? 'not closed — landing was not confirmed',
  boardEffects: closed?.boardEffects ?? [],
  findings: landed?.findings ?? [],
  reportMatchedTree: verifyOut.reportMatchesTree,
  filesStaged: landed?.filesStaged ?? [],
  suite: landed?.suite ?? [],
  verify: verifyOut.report,
  close: closed?.report ?? null,
}

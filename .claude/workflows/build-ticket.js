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
    { title: 'Land', detail: 'commit, merge to develop, push, close with evidence (/land skill)' },
  ],
}

// Green suite: ALWAYS the aliases from .claude/rules/verification.md.
// Skills /gate, /docs-sync, /land are the source of truth for process.

// args: { ticket: "GTW-123", slug: "editor-net-qa-passthrough" }
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

const HOUSE_RULES = `
## Standing rules — these override convenience and they are not negotiable

1. **NEVER drive the app with a script.** Drive the app ONLY through the \`mcp__gdtf-qa__*\` tools.
   If a tool is missing: STOP AND REPORT. Do NOT kill the resident process or rebuild gdtf_qa_mcp by hand.

2. **Pick the CHEAPEST SUFFICIENT evidence**: integration test ≈ unit test > reading the code >> building MCP tooling.

3. **Prove behaviour against the REAL binary.** MinimalPlugins + hand-inserted resources prove nothing.

4. **No unwrap/expect/panic/todo/unimplemented.** Doc every pub item. Typed domain values (no-bare-types.md).
   Files: warn >300 / block >400. mod.rs is wiring only.

5. **Comments.** See comment-hygiene.md — short docs, no ticket ids, no design rationale in comments.

6. **Symbols and bulk edits.** See code-navigation.md — the LSP tool answers every symbol
   question, \`rust-analyzer ssr\` makes the same change at many sites, and no script
   (Python, sed, awk, perl) ever edits Rust source.

7. **Plain language.** See plain-language.md — short plain wording; quoted failures stay whole.

8. **Report failures verbatim.** Never summarise a failure away.
`

const GREEN = `
## Green

READ \`${REPO}/.claude/rules/verification.md\` AND RUN THE SUITE IT LISTS. That file is the
only authority — this workflow does not restate the commands, because a copy here goes stale
the moment the rule changes. Do not run a suite from memory.

Use the \`.cargo/config.toml\` aliases exactly as written there. Never hand-type an expanded
feature list.

Run them SEQUENTIALLY, one command per tool call, and read each exit code on its own. PIN THE
DIRECTORY on every cargo call (\`cd ${REPO} && ...\`) — the Bash tool cwd resets between calls.

NEVER pipe a cargo command into \`tail\`/\`head\`/\`grep\` inside an \`&&\` chain. The pipeline's
exit code is the filter's, which always succeeds, so a failing step reports green. That exact
mistake produced a false all-green while \`cargo doc\` was exiting 101.

Green means every command in that file exited 0, after your final edit.
`

// --- Phase 0: clause audit (GTW-962) — BEFORE In Progress ---
// Never audit a summary argument. Only live Linear description + comments.
// Findings: orchestrator log only — no Linear comments.
// Example of a catch (GTW-882): report claimed verbatim acceptance while tree
// had a different reworded sentence; file list understated. Contradictory
// clauses in one ticket must be reported as a set, not reconciled silently.
phase('Clause-audit')

const liveTicket = await agent(`Read-only Linear fetch for ${TICKET} in project GDTF.

1. Fetch FULL description AND complete comment thread, verbatim.
2. Do NOT change status.
3. Report description and every comment verbatim.

Also report: labels, parent, blocking relationships.`,
  { model: 'opus', label: `fetch:${TICKET}`, phase: 'Clause-audit', agentType: 'project-manager' })

if (!liveTicket) throw new Error(`could not fetch ${TICKET} for clause audit`)

log(`${TICKET}: clause-audit input (live ticket text):\n${liveTicket}`)

const auditOut = await agent(`Clause audit for ${TICKET}. READ-ONLY. Do not touch Linear status or post comments.

You are given the LIVE ticket text (description + comments). Do not invent clauses from memory or from any summary string.

<live-ticket id="${TICKET}">
${liveTicket}
</live-ticket>

Check the acceptance clauses as a SET:

1. Every clause names evidence this run can actually produce (suite, git, MCP, file:line).
2. No two clauses contradict each other.
3. Every clause asserts something checkable (not pure aspiration).

Start with AUDIT_OK or AUDIT_BLOCK on its own line.
Then list findings. If zero checkers reported (you produce no real audit), that is failure — say AUDIT_BLOCK.

Historical shape of a catch (GTW-882): implementer prose disagreed with the tree; contradictory acceptance wording. Flag that class.

Do NOT post to Linear.`,
  { model: 'opus', label: `clause-audit:${TICKET}`, phase: 'Clause-audit' })

if (!auditOut) throw new Error(`${TICKET}: clause audit produced no report (empty reviewer) — abort`)

log(`${TICKET}: clause-audit findings:\n${auditOut}`)

const auditOk = (() => {
  const m = /\b(AUDIT_OK|AUDIT_BLOCK)\b/i.exec(auditOut)
  return !!m && m[1].toUpperCase() === 'AUDIT_OK'
})()

if (!auditOk) {
  return {
    ticket: TICKET,
    landed: false,
    reason: 'clause audit blocked before In Progress',
    audit: auditOut,
  }
}

const NOTES = typeof A?.notes === 'string' && A.notes.trim() ? `
## ORCHESTRATOR NOTES — from a prior clause audit

These are readings of clauses the audit found easy to get wrong. They do not add scope. If one
appears to contradict the ticket text above, the TICKET wins — and say so in your report.

${A.notes.trim()}
` : `
## CLAUSE AUDIT (this run)

${auditOut}
`

phase('Open')

const ticketText = await agent(`Linear status only for ${TICKET}.

1. Move ${TICKET} to **In Progress** NOW.
2. Re-fetch FULL description and complete comment thread, verbatim (status change may race with board edits).
3. Report description and every comment verbatim.

Also report: labels, parent, blocking relationships.`,
  { model: 'opus', label: `open:${TICKET}`, phase: 'Open', agentType: 'project-manager' })

if (!ticketText) throw new Error(`could not open ${TICKET}`)

phase('Build')

const built = await agent(`Implement ${TICKET} in the MAIN repo at ${REPO}.

## Set up — RESUMABLE

All work happens directly in ${REPO}, on ${BRANCH}. Never commit on develop; the
pre-commit hook blocks it.

FIRST check what is already checked out:
\`\`\`
git -C ${REPO} branch --show-current
git -C ${REPO} status --short
\`\`\`

- If ${BRANCH} is already checked out: adopt it. Never reset or discard unlanded work.
- If it exists but is not checked out: \`git -C ${REPO} checkout ${BRANCH}\`
- Otherwise, from a clean tree on develop:
  \`git -C ${REPO} checkout develop && git -C ${REPO} checkout -b ${BRANCH}\`

If the tree is dirty with work that is not this ticket's, STOP and report — do not
stash it and do not build on top of it.

One ticket at a time in this repo.

## THE CONTRACT — build exactly this

<ticket id="${TICKET}">
${ticketText}
</ticket>

Build every clause. Do not narrow. If a clause is impossible, STOP and report.
${NOTES}

## BEFORE YOU WRITE ANY CODE — bootstrap check

If a clause demands live evidence for a capability THIS TICKET is adding (courier change or protocol version bump), that clause is unsatisfiable. STOP and report.

${HOUSE_RULES}

${GREEN}

Run the suite yourself before reporting. Do NOT commit.

Report: files changed, suite result, clause-by-clause evidence.`,
  { model: 'opus', label: `build:${TICKET}`, phase: 'Build' })

if (!built) throw new Error(`build agent died on ${TICKET}`)

const fresh = await agent(`Re-fetch ${TICKET} from Linear. READ-ONLY. Return CURRENT full description and complete comment thread, verbatim.`,
  { model: 'opus', label: `refetch:${TICKET}`, phase: 'Verify', agentType: 'project-manager' })

const contract = fresh || ticketText
if (fresh && fresh !== ticketText) {
  log(`${TICKET}: contract re-fetched — verifying against the CURRENT ticket text`)
}

async function verify(attempt) {
  return agent(`Independently verify ${TICKET} in ${REPO}. Trust NOTHING the implementer reported.

<ticket id="${TICKET}">
${contract}
</ticket>

<implementer-report attempt="${attempt}">
${built}
</implementer-report>

${GREEN}

Reproduce every evidence clause yourself. Start with GREEN or RED on its own line.

${HOUSE_RULES}`,
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
  { key: 'fidelity', focus: `Does the diff implement EVERY clause exactly as written? Hunt silent narrowing. Check docs/ too.` },
  { key: 'tests', focus: `Do the tests exercise the REAL code path? Reject MinimalPlugins stand-ins. Name mutations that would slip past. Run ZERO cargo.` },
  { key: 'structure', focus: `Module layout, no bare types, no unwrap/todo, Bevy schedule/ordering. Run ZERO cargo.` },
]

async function gate(attempt) {
  const verdicts = await parallel(LENSES.map(l => () =>
    agent(`Adversarial read-only review of ${TICKET} in ${REPO}.

<ticket id="${TICKET}">
${contract}
</ticket>

## YOUR LENS
${l.focus}

## VERIFICATION EVIDENCE
<verify-report>
${verifyOut || '(verify died — treat cargo claims as UNPROVEN)'}
</verify-report>

${HOUSE_RULES}

Run ZERO cargo. Cite file:line. Start with COMPLIANT or NON-COMPLIANT.`,
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
const reviewersHeardFrom = () => (verifyOut ? 1 : 0) + verdicts.filter(Boolean).length

while (!allPass() && attempt < 5) {
  if (reviewersHeardFrom() === 0) {
    throw new Error(`${TICKET}: no reviewer reported (round ${attempt}). Infrastructure failure — resume.`)
  }
  attempt++
  phase('Fix')
  const problems = [
    readVerdict(verifyOut) ? null : `<verify-verdict>\n${verifyOut || '(died)'}\n</verify-verdict>`,
    ...verdicts.map((v, i) => compliant(v) ? null : `<gate-lens name="${LENSES[i].key}">\n${v || '(died)'}\n</gate-lens>`),
  ].filter(Boolean).join('\n\n')

  log(`${TICKET}: round ${attempt} — repairing`)

  const fixed = await agent(`Repair ${TICKET} in ${REPO}.

<ticket id="${TICKET}">
${contract}
</ticket>

## WHAT BLOCKED IT
${problems}
${NOTES}

Fix at root. Do NOT weaken tests or edit the gate. ${HOUSE_RULES}
${GREEN}

Re-run the suite. Do NOT commit.`,
    { model: 'opus', label: `fix:${TICKET}#${attempt}`, phase: 'Fix' })

  if (!fixed) throw new Error(`fix agent died on ${TICKET} round ${attempt}`)

  phase('Verify')
  verifyOut = await verify(attempt)
  phase('Gate')
  verdicts = await gate(attempt)
}

if (!allPass()) {
  return { ticket: TICKET, landed: false, reason: `still blocked after ${attempt} rounds`, verify: verifyOut, gate: verdicts }
}

phase('Docs-sync')

await agent(`Docs-sync posture for ${TICKET} in ${REPO}.

If the change touched behavior described in docs/, verify claims against the code and fix drift.
Code is authority for what exists; docs/ remain authority for design intent.
If nothing in docs/ drifted, say so explicitly and stop.
Do NOT commit — /land owns the commit.`,
  { model: 'opus', label: `docs-sync:${TICKET}`, phase: 'Docs-sync' })

phase('Land')

const landed = await agent(`Land ${TICKET} following the /land skill.

Repo: ${REPO} — no worktree; the work is on ${BRANCH} in the main tree
Branch: ${BRANCH}

## VERIFICATION EVIDENCE
<verify-report>
${verifyOut}
</verify-report>

## GATE VERDICTS (final round ${attempt})
${verdicts.map((v, i) => `<gate-lens name="${LENSES[i].key}">\n${v}\n</gate-lens>`).join('\n')}

Steps (see .claude/skills/land/SKILL.md):
1. Re-run full green suite if develop moved.
2. Write .claude/.gate-pass (TICKET/BRANCH/HEAD/FINGERPRINT).
3. Stage explicit files by name. Never git add -A.
4. Commit: Area: summary (${TICKET}). Confirm subject names the ticket.
5. From ${REPO}: checkout develop, pull, merge --no-ff ${BRANCH}, push origin develop, delete local feature branch.
6. Leave ${REPO} on develop with a clean tree.

If the merge conflicts, resolve it, stage the resolved files, and finish with
\`git merge --continue\` — never the commit subcommand. The pre-commit hook blocks
every commit on develop, including the one that concludes a conflicted merge.

Never --no-verify. Report what git said. Do not claim landing is proven —
a separate confirm step will check origin/develop.`,
  { model: 'opus', label: `land:${TICKET}`, phase: 'Land' })

function parseLandedCommit(text) {
  if (typeof text !== 'string' || !text.trim()) return null
  const hits = []
  for (const line of text.split(/\r?\n/)) {
    const m = line.trim().match(/^LANDED_COMMIT=([0-9a-f]{7,40}|NO)$/i)
    if (m) hits.push(m[1])
  }
  if (hits.length !== 1) return null
  if (hits[0].toUpperCase() === 'NO') return null
  return hits[0]
}

const confirmOut = await agent(`Confirm landing of ${TICKET}. Your ENTIRE reply must be exactly one line.

Main repo: ${REPO}
Ticket: ${TICKET}

Do this and nothing else:
1. git -C ${REPO} fetch origin develop
2. Find the commit on origin/develop whose subject contains ${TICKET} (most recent if several).
   If none, reply exactly: LANDED_COMMIT=NO
3. Run: git -C ${REPO} merge-base --is-ancestor <sha> origin/develop
4. Run: git -C ${REPO} ls-remote origin refs/heads/develop
5. If merge-base exits 0 and the SHA is reachable on origin/develop, reply exactly:
   LANDED_COMMIT=<sha>
   Otherwise reply exactly:
   LANDED_COMMIT=NO

No other text. No markdown. No log. One line only.`,
  { model: 'opus', label: `confirm-land:${TICKET}`, phase: 'Land' })

const landedSha = parseLandedCommit(confirmOut)
const landedOk = !!landedSha

if (landedOk) {
  await agent(`Close ${TICKET} in Linear with evidence.

Move In Review then Done. Comment BEFORE status change with: landing SHA ${landedSha}, suite result, evidence clauses satisfied.

<confirm-report>
${confirmOut}
</confirm-report>

<land-report>
${landed}
</land-report>

<verify-report>
${verifyOut}
</verify-report>`,
    { model: 'opus', label: `close:${TICKET}`, phase: 'Land', agentType: 'project-manager' })
}

return { ticket: TICKET, landed: landedOk, landedSha: landedSha || null, rounds: attempt, land: landed, confirm: confirmOut, verify: verifyOut }

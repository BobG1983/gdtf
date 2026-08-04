export const meta = {
  name: 'build-ticket',
  description: 'Build one GTW ticket end to end: open, build, verify, 3-lens gate, docs-sync, land, close. Uses the skills and the single green definition in .claude/rules/verification.md.',
  phases: [
    { title: 'Open', detail: 'fetch the ticket verbatim and mark it In Progress' },
    { title: 'Build', detail: 'implement against the quoted contract' },
    { title: 'Verify', detail: 'full green suite (verification.md) plus the ticket evidence clauses' },
    { title: 'Gate', detail: '3 read-only lenses, any-non-compliant blocks (/gate skill)' },
    { title: 'Fix', detail: 'bounded repair loop on a red verdict' },
    { title: 'Docs-sync', detail: 're-align docs/ if the change drifted design claims (/docs-sync skill)' },
    { title: 'Land', detail: 'commit, finish, push, close with evidence (/land skill)' },
  ],
}

// Green suite: ALWAYS the eight aliases from .claude/rules/verification.md.
// Skills /gate, /docs-sync, /land are the source of truth for process; this workflow
// orchestrates agents that follow those skills.

// args: { ticket: "GTW-123", slug: "editor-net-qa-passthrough" }
// Accept either a real object or a JSON-encoded string — the harness may deliver either.
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { throw new Error(`args was an unparseable string: ${A}`) }
}
const TICKET = A?.ticket
const SLUG = A?.slug
if (!TICKET || !SLUG) throw new Error(`args must supply { ticket, slug }; got ${JSON.stringify(args)}`)

const NOTES = typeof A?.notes === 'string' && A.notes.trim() ? `
## ORCHESTRATOR NOTES — from the clause audit already run on this ticket

These are readings of clauses the audit found easy to get wrong. They do not add scope. If one
appears to contradict the ticket text above, the TICKET wins — and say so in your report.

${A.notes.trim()}
` : ''

const REPO = '/Users/bgardner/dev/gdtf'
const BRANCH = `feature/${SLUG}`
const WT = `${REPO}/.claude/worktrees/${SLUG}`

const HOUSE_RULES = `
## Standing rules — these override convenience and they are not negotiable

1. **NEVER drive the app with a script.** Drive the app ONLY through the \`mcp__gdtf-qa__*\` tools.
   If a tool is missing: STOP AND REPORT. Do NOT kill the resident process or rebuild gdtf_qa_mcp by hand.

2. **Pick the CHEAPEST SUFFICIENT evidence**: integration test ≈ unit test > reading the code >> building MCP tooling.

3. **Prove behaviour against the REAL binary.** MinimalPlugins + hand-inserted resources prove nothing.

4. **No unwrap/expect/panic/todo/unimplemented.** Doc every pub item. Typed domain values (no-bare-types.md).
   Files: warn >300 / block >400. mod.rs is wiring only.

5. **Plain language.** Banned: "seam", "sanctioned", "byte identical". Name the real mechanism.

6. **Report failures verbatim.** Never summarise a failure away.
`

const GREEN = `
The one definition of green — authority: .claude/rules/verification.md.
ALL EIGHT must exit 0. Use the aliases. Never hand-type expanded feature lists.

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

Run SEQUENTIALLY, one command per tool call. PIN THE DIRECTORY on every cargo call
(\`cd ${WT} && ...\`). The Bash tool cwd resets between calls.
`

phase('Open')

const ticketText = await agent(`Read-only Linear work, then ONE status change.

1. Fetch ${TICKET} from the GDTF project: FULL description AND complete comment thread, verbatim.
2. Move ${TICKET} to **In Progress** NOW.
3. Report description and every comment verbatim.

Also report: labels, parent, blocking relationships.`,
  { model: 'opus', label: `open:${TICKET}`, phase: 'Open', agentType: 'project-manager' })

if (!ticketText) throw new Error(`could not fetch ${TICKET}`)

phase('Build')

const built = await agent(`Implement ${TICKET} in a git worktree.

## Set up — RESUMABLE

FIRST check whether the worktree already exists:
\`\`\`
git -C ${REPO} worktree list
\`\`\`

- If ${WT} is NOT listed: git -C ${REPO} worktree add -b ${BRANCH} ${WT} develop
- If ${WT} IS listed: adopt it. Never delete or reset finished unlanded work.

Work ONLY in ${WT}.

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
  return agent(`Independently verify ${TICKET} in ${WT}. Trust NOTHING the implementer reported.

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
    agent(`Adversarial read-only review of ${TICKET} in ${WT}.

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

  const fixed = await agent(`Repair ${TICKET} in ${WT}.

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

// After gate passes, run docs-sync posture before land (skill: /docs-sync).
await agent(`Docs-sync posture for ${TICKET} in ${WT}.

If the change touched behavior described in docs/, verify claims against the code and fix drift.
Code is authority for what exists; docs/ remain authority for design intent.
If nothing in docs/ drifted, say so explicitly and stop.
Do NOT commit — /land owns the commit.`,
  { model: 'opus', label: `docs-sync:${TICKET}`, phase: 'Docs-sync' })

phase('Land')

const landed = await agent(`Land ${TICKET} following the /land skill.

Worktree: ${WT}
Branch: ${BRANCH}
Main repo: ${REPO}

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
5. git flow feature finish from ${REPO}.
6. Push develop. Confirm with git ls-remote.
7. Clean up worktree/branch.

Never --no-verify. Report commit SHA, push result, cleanup.`,
  { model: 'opus', label: `land:${TICKET}`, phase: 'Land' })

const landedOk = !!landed && /[0-9a-f]{7,40}/.test(landed)

if (landedOk) {
  await agent(`Close ${TICKET} in Linear with evidence.

Move In Review then Done. Comment BEFORE status change with: landing SHA, suite result, evidence clauses satisfied.

<land-report>
${landed}
</land-report>

<verify-report>
${verifyOut}
</verify-report>`,
    { model: 'opus', label: `close:${TICKET}`, phase: 'Land', agentType: 'project-manager' })
}

return { ticket: TICKET, landed: landedOk, rounds: attempt, land: landed, verify: verifyOut }

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
    { title: 'Land', detail: 'file any carried fix, commit, rebase onto develop, fast-forward, push, summarize, close (/land skill)' },
  ],
}

// Green suite: ALWAYS the aliases from .claude/rules/verification.md.
// Skills /gate, /docs-sync, /land are the source of truth for process.
//
// Every agent answers through a schema. Nothing here reads a verdict out of prose —
// that cost repair rounds on correct code when a report opened "NON-COMPLIANT? No — COMPLIANT".

// Model tier per call — the rubric is .claude/rules/model-tiering.md (user ruling 2026-08-18).
// Very hard or creative -> fable. Hard, engineering-focused -> opus. Mechanical copy/compare ->
// sonnet. Between tiers -> the higher one.
//
//   fetch:*         sonnet  verbatim Linear snapshot, judges nothing
//   clause-audit:*  opus    audit judgment against tree and canon
//   open:*          sonnet  status move plus templated comment
//   build:*         opus    the implementation
//   refetch:*       sonnet  read-only re-fetch
//   verify:*        sonnet  adversarial verification, trusts nothing
//   gate:*          opus    design-gate lens
//   fix:*           opus    repair engineering
//   docs-sync:*     opus    decides doc drift against source
//   file-carried:*  sonnet  files the carried fix's ticket from text the builder wrote
//   land:*          sonnet  rebase, suite, and the git facts only
//   summarize:*     sonnet  gathers its own evidence; owns every judged field of the result
//   confirm-land:*  sonnet  compares expected landing state to actual
//   close:*         sonnet  posts what summarize wrote and moves status; judges nothing
//
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

// A comment written through the Linear MCP wears the account owner's name, so authorship is
// readable only from the `**[agent]**` source line linear-discipline.md requires. An unheadered
// comment is one the owner typed into Linear themselves — the only kind that can carry a ruling.
const TICKET_SNAPSHOT = {
  type: 'object', additionalProperties: false,
  required: ['description', 'comments', 'userNotes', 'labels', 'state', 'parent', 'relations'],
  properties: {
    description: { type: 'string', description: 'The full description, verbatim. Never a summary.' },
    comments: {
      type: 'array', description: 'Every comment, oldest first, verbatim and none omitted.',
      items: {
        type: 'object', additionalProperties: false, required: ['source', 'body'],
        properties: {
          source: { type: 'string', description: 'The `**[name]**` source line without brackets — clause-audit, project-manager, build-ticket / land. Exactly "owner" when the comment has no source line.' },
          body: { type: 'string', description: 'The comment verbatim, minus the source line.' },
        },
      },
    },
    userNotes: {
      type: 'array', items: { type: 'string' },
      description: 'Bodies of the comments whose source is "owner", repeated here because these are the only ones that can hold a ruling. Empty if none.',
    },
    labels: { type: 'array', items: { type: 'string' }, description: 'Label names as the board shows them.' },
    state: { type: 'string', description: 'The workflow state name right now.' },
    parent: { type: 'string', description: 'Parent as `GTW-n title`, or empty if none.' },
    relations: {
      type: 'array', description: 'Fetched with includeRelations: true — without it the board returns none and a ticket with edges looks like one without.',
      items: {
        type: 'object', additionalProperties: false, required: ['kind', 'ticket', 'title'],
        properties: {
          kind: { type: 'string', enum: ['blocks', 'blocked-by', 'related-to', 'duplicate-of', 'duplicated-by', 'other'] },
          ticket: { type: 'string', description: 'GTW-n' },
          title: { type: 'string' },
        },
      },
    },
  },
}

// The corrections are the reason phase 0 exists. As prose they can be skimmed past or half-applied
// and nothing downstream can tell which; as rows the builder gets a list it must answer one by one.
const AUDIT_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'corrections', 'blockingQuestion', 'report'],
  properties: {
    verdict: { type: 'string', enum: ['AUDIT_OK', 'AUDIT_BLOCK'], description: 'AUDIT_BLOCK for a product decision the code cannot answer, or for a sweep ticket whose site list has no guard command behind it.' },
    corrections: {
      type: 'array', description: 'One row per clause you corrected. Empty when the ticket needed none — never a row saying a clause was fine.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'problem', 'replacementText'],
        properties: {
          clause: { type: 'string', description: 'The clause number as the ticket numbers it, e.g. "5".' },
          problem: { type: 'string', description: 'What is wrong with it as written — the missing symbol, the deleted guard, the evidence that cannot be produced.' },
          replacementText: { type: 'string', description: 'The clause rewritten in full, ready to build against. Corrects facts; never cuts a requirement.' },
        },
      },
    },
    blockingQuestion: { type: 'string', description: 'On AUDIT_BLOCK, what to put to the owner: the product decision, or the command a sweep ticket\'s site list must come from. Empty on AUDIT_OK.' },
    report: { type: 'string', description: 'Everything the rows do not carry: what you opened, what you checked, why a clause stands as written.' },
  },
}

// One row per suite command, shared so the implementer and the verifier report exit codes the
// same way — the verifier is the one that must be machine-checkable, since its whole job is to
// distrust the implementer.
const SUITE_ROWS = {
  type: 'array', description: 'One row per suite command, with the exit code you read yourself. Every command in verification.md, none omitted.',
  items: {
    type: 'object', additionalProperties: false, required: ['command', 'exit'],
    properties: { command: { type: 'string' }, exit: { type: 'integer' } },
  },
}

// Everything the run has to check later is a field. Prose in `report` cannot be asserted against,
// and the two things that went wrong most — a correction nobody answered and a quiet narrowing
// reported as success — both hid in prose.
const WORK_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['filesChanged', 'suite', 'clauses', 'corrections', 'tests', 'deviations', 'carriedFix', 'foreignDirtyFiles', 'outOfScope', 'report'],
  properties: {
    filesChanged: { type: 'array', items: { type: 'string' }, description: 'Every path you created or edited, by name, exactly as git reports it. Empty only if you truly changed nothing.' },
    suite: SUITE_ROWS,
    clauses: {
      type: 'array', description: 'One row per clause in the contract, none omitted.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'status', 'symbols', 'evidence'],
        properties: {
          clause: { type: 'string', description: 'The clause number as the contract numbers it.' },
          status: { type: 'string', enum: ['built', 'already-true', 'blocked'], description: 'already-true means the tree satisfied it before you touched it. Say that rather than claiming work you did not do.' },
          symbols: { type: 'array', items: { type: 'string' }, description: 'Symbols you added or changed for this clause, each with the file holding it.' },
          evidence: { type: 'string', description: 'What shows the clause holds. A blocked clause states what stopped you.' },
        },
      },
    },
    corrections: {
      type: 'array', description: 'One row per correction the clause audit handed you — every one, including corrections that needed no work.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'applied', 'how'],
        properties: {
          clause: { type: 'string' },
          applied: { type: 'boolean' },
          how: { type: 'string', description: 'Where it landed, or why it needed nothing.' },
        },
      },
    },
    tests: {
      type: 'array', description: 'Tests you added or changed.',
      items: {
        type: 'object', additionalProperties: false, required: ['name', 'file', 'clause', 'catches'],
        properties: {
          name: { type: 'string' },
          file: { type: 'string' },
          clause: { type: 'string', description: 'The clause this test proves.' },
          catches: { type: 'string', description: 'The mutation this test turns red. A test with no answer here does not discriminate, so do not write it.' },
        },
      },
    },
    deviations: {
      type: 'array', description: 'Anything built differently from the contract. Empty is the expected value — design-fidelity.md makes a silent deviation a defect.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'built', 'why'],
        properties: { clause: { type: 'string' }, built: { type: 'string' }, why: { type: 'string' } },
      },
    },
    carriedFix: {
      type: 'object', additionalProperties: false, required: ['files', 'clause', 'defect', 'title'],
      description: 'One defect you fixed because it blocked a clause of THIS ticket, under the carried fix section of design-fidelity.md. An empty array and empty strings when you carried none, which is the common case. You still do not commit: a project-manager step files the ticket and the land step makes the commit.',
      properties: {
        files: { type: 'array', items: { type: 'string' }, description: 'The paths the fix touched. Every one is also in filesChanged, because the tree holds both. Land stages these on their own, so a path missing here lands under the wrong ticket.' },
        clause: { type: 'string', description: 'The clause of THIS ticket the defect blocked. A defect that blocked no clause is not carried: name it in outOfScope instead.' },
        defect: { type: 'string', description: 'What was broken and where, by symbol or quoted text, and what the fix does. A project-manager step files this verbatim as the ticket description.' },
        title: { type: 'string', description: 'The title for that ticket, in the style linear-discipline.md asks for. The step that files it copies text and judges nothing, so the wording is yours.' },
      },
    },
    foreignDirtyFiles: { type: 'array', items: { type: 'string' }, description: 'Files dirty in the tree that are not this ticket\'s work. You left them alone; naming them is what stops land from staging them.' },
    outOfScope: { type: 'array', items: { type: 'string' }, description: 'Ticket-worthy findings outside this contract. You cannot file them; the orchestrator does.' },
    report: { type: 'string', description: 'Only what the rows do not carry.' },
  },
}

const VERIFY_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'suite', 'clauses', 'reportMatchesTree', 'unansweredCorrections', 'undeclaredDeviations', 'report'],
  properties: {
    verdict: { type: 'string', enum: ['GREEN', 'RED'], description: 'GREEN only when every suite command exited 0 after the final edit AND every evidence clause reproduced.' },
    suite: SUITE_ROWS,
    clauses: {
      type: 'array', description: 'One row per clause you reproduced. A clause you could not reach is met: false with the reason as evidence.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'met', 'evidence'],
        properties: {
          clause: { type: 'string', description: 'The clause number as the ticket numbers it.' },
          met: { type: 'boolean' },
          evidence: { type: 'string', description: 'What you ran or read, and what it showed. A failure is quoted whole, never summarised.' },
        },
      },
    },
    reportMatchesTree: { type: 'boolean', description: 'Did the implementer\'s filesChanged list match `git status --porcelain`? False if it named files it did not touch, or missed files it did.' },
    unansweredCorrections: {
      type: 'array', items: { type: 'string' },
      description: 'Clause numbers whose audit correction the implementer left out of its corrections rows, or claimed applied where the tree says otherwise. Empty when every correction is honestly accounted for.',
    },
    undeclaredDeviations: {
      type: 'array', description: 'Places the code differs from the contract that the implementer did not declare. A declared deviation belongs to the gate; an undeclared one is what this field exists to catch.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'differs'],
        properties: { clause: { type: 'string' }, differs: { type: 'string', description: 'What the contract asks and what the code does, with the symbol.' } },
      },
    },
    report: { type: 'string', description: 'Everything the rows do not carry.' },
  },
}

// verdict and findings are cross-checked: a COMPLIANT verdict carrying findings is treated as
// NON-COMPLIANT. A review that says both cannot be read as the safer one by accident.
const LENS_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'findings', 'report'],
  properties: {
    verdict: { type: 'string', enum: ['COMPLIANT', 'NON-COMPLIANT'], description: 'NON-COMPLIANT only for a clause you can show is unmet. Must be NON-COMPLIANT whenever findings is non-empty.' },
    findings: {
      type: 'array', description: 'One row per unmet clause. Empty on COMPLIANT. Not a place for notes, preferences, or work outside this ticket.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'symbol', 'file', 'quote', 'why'],
        properties: {
          clause: { type: 'string', description: 'The clause number this violates.' },
          symbol: { type: 'string', description: 'The function, type or test the repair agent must open. Never a bare line number.' },
          file: { type: 'string', description: 'Path holding that symbol.' },
          quote: { type: 'string', description: 'The line of code or docs that shows the violation, copied exactly.' },
          why: { type: 'string', description: 'Why that line fails that clause, and what would satisfy it. Write it for the agent that has to repair the code.' },
        },
      },
    },
    report: { type: 'string', description: 'What you opened and traced to reach the verdict, including the clauses you found met.' },
  },
}

// The claims it checked and found correct are the only evidence of coverage, so they are rows too.
// `conflicts` exists because design-fidelity.md rule 4 forbids picking a side when code and docs
// disagree — without a field for it, "I stopped and it needs a ruling" has nowhere to go.
const DOCS_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['scope', 'claims', 'conflicts', 'filesChanged', 'report'],
  properties: {
    scope: { type: 'array', items: { type: 'string' }, description: 'Docs you judged in range for this change, by path — including any you opened and found untouched by it.' },
    claims: {
      type: 'array', description: 'One row per written claim you checked against the code. Rows for claims that already matched are what shows the sweep happened.',
      items: {
        type: 'object', additionalProperties: false, required: ['doc', 'claim', 'verdict', 'evidence'],
        properties: {
          doc: { type: 'string', description: 'Path of the doc holding the claim.' },
          claim: { type: 'string', description: 'The written claim, quoted.' },
          verdict: { type: 'string', enum: ['matches', 'fixed', 'conflict'], description: 'matches: the code agrees. fixed: you rewrote the doc. conflict: code and docs disagree on design intent and you stopped.' },
          evidence: { type: 'string', description: 'The symbol or code that settles it, and what you rewrote if anything.' },
        },
      },
    },
    conflicts: {
      type: 'array', description: 'Code and docs disagreeing on intent, which you must not resolve yourself. Empty is the usual value.',
      items: {
        type: 'object', additionalProperties: false, required: ['doc', 'docSays', 'codeDoes'],
        properties: { doc: { type: 'string' }, docSays: { type: 'string' }, codeDoes: { type: 'string' } },
      },
    },
    filesChanged: { type: 'array', items: { type: 'string' }, description: 'Docs you edited, by name. Empty when nothing drifted.' },
    report: { type: 'string', description: 'Only what the rows do not carry.' },
  },
}

// The land agent holds no Linear tools, so it has no field to guess with. Landing is
// proven by the confirm step and a first-hand git check, never by this report.
//
// It reports only what git printed. Every judged field belongs to the summarize step.
const LAND_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['committed', 'commitSha', 'pushedRange', 'filesStaged', 'filesLeftUnstaged',
    'carriedFixCommit', 'carriedCommits', 'rebase', 'conflictedFiles', 'developBefore',
    'developAfter', 'gatePass', 'branchDeleted', 'suite'],
  properties: {
    committed: { type: 'boolean', description: 'Did the commit and push actually run' },
    commitSha: { type: 'string', description: 'Feature commit sha, or empty if none. Empty with committed true is a contradiction the run rejects.' },
    pushedRange: { type: 'string', description: 'Exactly what git push printed, or empty' },
    filesStaged: { type: 'array', items: { type: 'string' }, description: 'Every path staged, by name. Never a glob or a count.' },
    filesLeftUnstaged: { type: 'array', items: { type: 'string' }, description: 'Paths dirty in the tree you deliberately did not stage. "I left them alone" is checkable only if you name them.' },
    carriedFixCommit: {
      type: 'object', additionalProperties: false, required: ['sha', 'subject', 'ticket'],
      description: 'The commit you made for the carried fix the run handed you. Empty strings when the run handed you none.',
      properties: {
        sha: { type: 'string' },
        subject: { type: 'string', description: 'The subject line you wrote, so the ticket in it can be read rather than assumed.' },
        ticket: { type: 'string', description: 'The GTW id that subject names. It is the carried fix\'s own ticket, never this one.' },
      },
    },
    carriedCommits: {
      type: 'array', description: 'Commits in `develop..branch` whose subject does not name this ticket, and that nobody in this run expected. Read them from git log, not from memory. They reach develop with this land whether or not they belong to it. The carried fix commit you made is expected, so it goes in carriedFixCommit and not here.',
      items: { type: 'object', additionalProperties: false, required: ['sha', 'subject'], properties: { sha: { type: 'string' }, subject: { type: 'string' } } },
    },
    rebase: { type: 'string', enum: ['clean', 'conflicts-resolved', 'stopped', 'not-needed'], description: 'What the rebase onto origin/develop actually did.' },
    conflictedFiles: { type: 'array', items: { type: 'string' }, description: 'Files you resolved during the rebase. Empty unless rebase is conflicts-resolved.' },
    developBefore: { type: 'string', description: 'develop sha before the fast-forward.' },
    developAfter: { type: 'string', description: 'develop sha after the push. The pair proves the fast-forward.' },
    gatePass: {
      type: 'object', additionalProperties: false, required: ['fingerprint', 'scope', 'head'],
      description: 'What you wrote into .claude/.gate-pass, so the file and this report can be compared.',
      properties: { fingerprint: { type: 'string' }, scope: { type: 'string' }, head: { type: 'string' } },
    },
    branchDeleted: { type: 'boolean', description: 'Was the local feature branch deleted. A stale branch is what the next tick reads as live work.' },
    suite: SUITE_ROWS,
  },
}

// The only agent in the run that fetches origin, so it is also the cheapest place to ask what else
// came with the push. On GTW-1159 two unrelated commits rode in and nobody asked.
const CONFIRM_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['landedSha', 'subject', 'developSha', 'carriedFixSha', 'carriedFixSubject',
    'extraCommits', 'gatePass'],
  properties: {
    gatePass: {
      type: 'object', additionalProperties: false, required: ['matchedReport', 'deleted', 'note'],
      description: 'Step 7 of the land skill, which this step owns: compare .claude/.gate-pass against what the land step reported, then delete it.',
      properties: {
        matchedReport: { type: 'boolean', description: 'Whether the file on disk matched the land step\'s reported branch, head, fingerprint and scope.' },
        deleted: { type: 'boolean', description: 'Whether the file is gone now. False is correct when the land was not proven — say so in note.' },
        note: { type: 'string', description: 'What differed, or why the file was left alone. Empty when it matched and was deleted.' },
      },
    },
    landedSha: { type: 'string', description: 'The sha on origin/develop whose subject names this ticket, or the empty string if there is none.' },
    subject: { type: 'string', description: 'That commit\'s subject line, so the sha is checkable against the ticket rather than assumed.' },
    developSha: { type: 'string', description: 'origin/develop head after the fetch.' },
    carriedFixSha: { type: 'string', description: 'The sha on origin/develop whose subject names the carried fix ticket the prompt gave you, or the empty string when the prompt named no ticket or no such commit is on origin/develop. The rebase rewrites shas, so this is the only sha the run can report for that commit.' },
    carriedFixSubject: { type: 'string', description: 'That commit\'s subject line, so the sha is checkable against the carried ticket rather than assumed. Empty whenever carriedFixSha is.' },
    extraCommits: {
      type: 'array', description: 'Commits pushed in the same range whose subject does not name this ticket, apart from the carried fix commit you reported in carriedFixSha. Empty is the expected value.',
      items: { type: 'object', additionalProperties: false, required: ['sha', 'subject'], properties: { sha: { type: 'string' }, subject: { type: 'string' } } },
    },
  },
}

// The only agent in the run that can see the board. It posts what summarize wrote and moves the
// status; it judges nothing, so it has no summary field of its own.
const CLOSE_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['closed', 'ticketState', 'commentId', 'labelsAfter', 'boardEffects', 'report'],
  properties: {
    closed: { type: 'boolean', description: 'Is the ticket Done on the board now' },
    ticketState: { type: 'string', description: 'The state name the board shows for this ticket now, whatever it is.' },
    commentId: { type: 'string', description: 'Id the board returned for the evidence comment. Empty means no comment exists — say so rather than describing one.' },
    labelsAfter: { type: 'array', items: { type: 'string' }, description: 'Labels on the ticket after the close. A process flag left on — Needs User Input, Needs Splitting — is visible here and nowhere else.' },
    boardEffects: {
      type: 'array', items: { type: 'string' },
      description: 'Anything the close changed beyond this ticket — a child auto-completed, a parent auto-cancelled, a relation that vanished. Empty if none. Check children before and after.',
    },
    report: { type: 'string', description: 'What you posted and what the board said back.' },
  },
}

// --- Rendering -------------------------------------------------------------
// The agents downstream read prose, so the structured rows are rendered back into text at the
// point of use. Structure earns its keep before that: the board's fields arrive as fields, a
// correction cannot be half-quoted, and a verdict can be checked against the rows behind it.

function renderTicket(snap) {
  if (!snap) return '(ticket text unavailable)'
  const rel = snap.relations?.length
    ? snap.relations.map(r => `- ${r.kind}: ${r.ticket} ${r.title}`).join('\n')
    : '- none'
  const notes = snap.userNotes?.length
    ? snap.userNotes.map(n => `> ${n.replace(/\n/g, '\n> ')}`).join('\n\n')
    : '(none — nothing on this ticket is a ruling by the owner)'
  const comments = snap.comments?.length
    ? snap.comments.map(c => `### comment [${c.source}]\n\n${c.body}`).join('\n\n')
    : '(no comments)'
  return `## Description

${snap.description}

## Board

- state: ${snap.state || '(unknown)'}
- labels: ${snap.labels?.join(', ') || '(none)'}
- parent: ${snap.parent || '(none)'}

Relations:
${rel}

## Owner notes — written by the owner in Linear, so these are rulings

${notes}

## Comments, oldest first

${comments}`
}

// Every comment written through the MCP wears the owner's name, so a source line of "owner" is the
// only mark of a real ruling. Rendered separately above so a builder cannot mistake an agent's own
// note for one.
function renderCorrections(a) {
  if (!a?.corrections?.length) return '(the audit corrected no clause; build the ticket exactly as written)'
  return a.corrections.map(c => `### Clause ${c.clause}

**Wrong as written:** ${c.problem}

**Build this instead:**

${c.replacementText}`).join('\n\n')
}

function renderDeviations(work) {
  const ds = work?.deviations ?? []
  if (!ds.length) return '<declared-deviations>\n(none declared)\n</declared-deviations>'
  return `<declared-deviations>
${ds.map(d => `- clause ${d.clause}\n  built: ${d.built}\n  why it could not be built as written: ${d.why}`).join('\n')}
</declared-deviations>`
}

// A carried fix touches files no clause asks for, so a reviewer told nothing reads them as an
// undeclared deviation or as scope creep. design-fidelity.md says a declared carried fix is neither.
function renderCarriedFix(work) {
  const c = work?.carriedFix
  if (!c?.files?.length) return '<carried-fix>\n(none declared)\n</carried-fix>'
  return `<carried-fix clause="${c.clause}">
files: ${c.files.join(', ')}
title: ${c.title}
the defect: ${c.defect}

The carried fix section of .claude/rules/design-fidelity.md allows this: a defect blocking a clause
of this ticket is fixed by the build, filed as its own ticket by a project-manager step, and
committed on its own at land. Declared here, it is not an undeclared deviation and not out-of-scope
work. Judge whether it blocks the clause it names, and judge the rest of the diff as usual.
</carried-fix>`
}

function renderFindings(v, lensKey) {
  if (!v) return `<gate-lens name="${lensKey}">\n(died — nothing to act on)\n</gate-lens>`
  const rows = v.findings?.length
    ? v.findings.map(f => `- clause ${f.clause} — \`${f.symbol}\` in ${f.file}\n  quoted: ${f.quote}\n  why: ${f.why}`).join('\n')
    : '(verdict NON-COMPLIANT with no findings row — treat the report as the finding)'
  return `<gate-lens name="${lensKey}" verdict="${v.verdict}">
${rows}

${v.report}
</gate-lens>`
}

// A suite row set is green only if it is non-empty and every row exited 0. Empty means the agent
// ran nothing, which is not a pass.
const suiteGreen = (rows) => Array.isArray(rows) && rows.length > 0 && rows.every(r => r.exit === 0)

// --- Prompt fragments ------------------------------------------------------

const HOUSE_RULES = `
## Standing rules — not negotiable

1. Drive the app ONLY through the \`mcp__gdtf-mcp__*\` tools. Never a script, never a socket.
   A missing tool is a STOP-and-report, not a reason to rebuild or kill anything by hand.
2. Cheapest sufficient evidence: integration test ≈ unit test > reading the code >> building tooling.
3. Prove behaviour against the REAL binary. MinimalPlugins with hand-inserted resources proves nothing.
4. No unwrap/expect/panic/todo/unimplemented. Doc every pub item. Typed domain values (no-bare-types.md).
   Files: block >400. mod.rs is wiring only.
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

Fill every field of the schema:

- \`description\` and each \`comments[].body\` verbatim — never a summary, nothing omitted.
- \`comments[].source\` is the \`**[name]**\` line the comment opens with, brackets stripped. A
  comment with no such line was typed into Linear by the owner: its source is exactly \`owner\`,
  and its body goes in \`userNotes\` as well.
- \`relations\` needs \`includeRelations: true\`. Without it the board returns none, and a ticket
  with edges is indistinguishable from one without.`,
  { model: 'sonnet', label: `fetch:${TICKET}`, phase: 'Clause-audit', agentType: 'project-manager', schema: TICKET_SNAPSHOT })

if (!live) throw new Error(`could not fetch ${TICKET} for clause audit`)

const audit = await agent(`Clause audit for ${TICKET}.

Return one \`corrections\` row per clause you correct: the clause number, what is wrong with it as
written, and the whole clause rewritten ready to build against. A correction fixes facts and never
cuts a requirement, so a row that shrinks scope is a wrong row. Correct nothing and the array is
empty. Reasoning that belongs to no single clause goes in \`report\`.

<live-ticket id="${TICKET}">
${renderTicket(live)}
</live-ticket>`,
  { model: 'opus', label: `clause-audit:${TICKET}`, phase: 'Clause-audit', agentType: 'clause-audit', schema: AUDIT_RESULT })

if (!audit) throw new Error(`${TICKET}: clause audit died — abort`)

log(`${TICKET}: clause audit ${audit.verdict}`)

if (audit.verdict !== 'AUDIT_OK') {
  return {
    ticket: TICKET, landed: false, reason: 'clause audit blocked before In Progress',
    blockingQuestion: audit.blockingQuestion || '(none stated — read the audit)',
    corrections: audit.corrections ?? [],
    audit: audit.report,
  }
}

phase('Open')

// The audit's corrections are what the builder builds to, and they live nowhere but this run.
// Post them so a reader of the ticket can see what was actually built against, and so a run that
// dies after the audit does not take them with it.
const opened = await agent(`Linear status and one comment for ${TICKET}.

1. Post the comment below FIRST, before any status change.
2. Move ${TICKET} to **In Progress**.
3. Re-fetch the FULL description and complete comment thread (the status change may race with board edits).
4. Fill every schema field from that re-fetch, on the same terms as the first fetch: verbatim
   bodies, \`source\` taken from the \`**[name]**\` line or exactly \`owner\` when there is none,
   owner-written bodies repeated in \`userNotes\`, and \`relations\` with \`includeRelations: true\`.
   The comment you just posted is part of the thread you return.

The comment is the clause audit's corrections, posted verbatim under a source line so nobody reads
them as the owner's. Per \`.claude/rules/linear-discipline.md\` it opens with the source line and
nothing above it. Post exactly this, changing nothing inside it:

---
**[clause-audit]**

Corrections applied to this ticket's clauses for the build starting now. These are the audit's
readings, not a ruling by the owner. The build was made against them wherever they differ from the
description above.

${renderCorrections(audit)}

${audit.report}
---`,
  { model: 'sonnet', label: `open:${TICKET}`, phase: 'Open', agentType: 'project-manager', schema: TICKET_SNAPSHOT })

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
${renderTicket(opened)}
</ticket>

## CLAUSE AUDIT — every correction below is BINDING

The audit opened every file the ticket cites and checked it against the tree. Where a clause was
wrong it wrote a corrected one. **Build the corrected clauses** wherever the two differ; everything
it did not correct stands as the ticket wrote it. Answer each correction in your report by clause
number — a correction you neither built nor named is the failure this section exists to stop.

${renderCorrections(audit)}

${audit.report}

Build every clause. Do not narrow. If a clause is impossible, STOP and report.

If a clause demands live evidence for a capability THIS ticket is adding — a change to the MCP
server (\`mcp\`) or a protocol version bump — that clause cannot be met. STOP and report.

${HOUSE_RULES}
${GREEN}

Run the suite yourself before reporting. Do NOT commit.

## Reporting

\`filesChanged\` must match \`git status --porcelain\` exactly. Verify checks it against the tree.

One \`clauses\` row per clause, none omitted. A clause the tree already satisfied is \`already-true\`
— say that rather than claiming work you did not do.

One \`corrections\` row per correction the audit handed you, including the ones that needed no work.
Verify checks these against the tree, so a row claiming applied where nothing changed is worse than
an honest false.

Every test you add or change gets a \`tests\` row naming the mutation it turns red. If you cannot
name one, the test does not discriminate — do not write it.

\`deviations\` is expected to be empty. A clause you could not build as written goes there with the
reason, and a gate lens judges whether it was forced; leaving it out and reporting success is a
defect under design-fidelity.md.

A defect that blocks a clause of THIS ticket may be fixed in this run instead of filed and left, on
the four bounds in the carried fix section of \`.claude/rules/design-fidelity.md\`. Declare it in
\`carriedFix\`: the files, the clause it blocked, what was broken, and the title its ticket should
carry. A project-manager step files that ticket and the land step commits those files on their own
under it. You still do not commit, and a defect that blocks no clause goes in \`outOfScope\` instead.

Dirty files that are not this ticket's work go in \`foreignDirtyFiles\`. Anything ticket-worthy you
noticed outside this contract goes in \`outOfScope\` — you cannot file it, the orchestrator does.`,
  { model: 'opus', label: `build:${TICKET}`, phase: 'Build', agentType: 'engineer', schema: WORK_RESULT })

if (!built) throw new Error(`build agent died on ${TICKET}`)

// The contract everyone downstream judges against: the live ticket text PLUS the audit's
// corrections. The corrections are never written back to Linear, so a re-fetch alone loses them —
// and then the builder builds to one contract while verify and the lenses judge another. On
// GTW-1012 that cost five gate rounds and a non-landing. The audit goes LAST so it wins on conflict.
const fresh = await agent(`Re-fetch ${TICKET} from Linear. READ-ONLY.

Return the CURRENT state of every schema field: description and comment bodies verbatim, \`source\`
from each comment's \`**[name]**\` line or exactly \`owner\` where there is none, owner-written
bodies repeated in \`userNotes\`, and \`relations\` fetched with \`includeRelations: true\`.`,
  { model: 'sonnet', label: `refetch:${TICKET}`, phase: 'Verify', agentType: 'project-manager', schema: TICKET_SNAPSHOT })

const contract = `${renderTicket(fresh || opened)}

## CLAUSE AUDIT (this run) — every correction below is BINDING and OVERRIDES the text above

The corrections were NOT written back to Linear, so the text above is the uncorrected original.
Where they differ, the corrections win — the implementer built to them. Do NOT report a violation
for failing to do something a correction struck out, nor for doing what a correction requires.

${renderCorrections(audit)}

${audit.report}`

// --- Verify and gate -------------------------------------------------------

// One lens per question, and each owns its own failure modes. They used to share a checklist in
// the agent definition, which is how three reviews came back saying the same thing.
const LENSES = [
  {
    key: 'clauses', focus: `Is every clause true of the code? Open each one and trace it — "the report says so" is not evidence.
Hunt quiet narrowing: a clause half-built reads as built. Reject hedge markers — TODO, FIXME, "for now",
"placeholder", "stub", "simplified". Reject any system, plugin or resource the ticket claims runs that
nothing registers. Check docs/ against the same clauses.` },
  {
    key: 'tests', focus: `Is the behaviour actually proven? Every behavioural clause needs a real-path,
assertion-bearing test that discriminates — name the mutation that would slip past each one. A clause with
no test is NON-COMPLIANT, not a note. Reject MinimalPlugins stand-ins where the claim is about the real app,
and reject exact-magnitude asserts on tunable data. Run ZERO cargo.` },
  {
    key: 'deviations', focus: `Are the implementer's declared deviations legitimate? Judge each one on its
own. COMPLIANT means every deviation is forced: the contract asks for something that does not exist,
contradicts itself, or contradicts another clause, and what was built serves the same purpose and is proven
by the same mutation. Open the API, the crate source, or the clauses said to conflict, and confirm it — the
implementer's reasoning is a claim, not evidence. NON-COMPLIANT means at least one deviation is avoidable:
the contract could have been built as written, or what was built does not do the same job. Give that one a
findings row. Run ZERO cargo.` },
  {
    key: 'rules', focus: `Does it obey the house? no-bare-types, module-layout (including files over 400 lines
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

<implementer-clause-claims>
${(work.clauses ?? []).map(c => `${c.clause} [${c.status}] ${c.symbols?.join(', ') || '(no symbol named)'} — ${c.evidence}`).join('\n') || '(none claimed)'}
</implementer-clause-claims>

<implementer-corrections-answered>
${(work.corrections ?? []).map(c => `${c.clause} applied=${c.applied} — ${c.how}`).join('\n') || '(none answered)'}
</implementer-corrections-answered>

<implementer-tests>
${(work.tests ?? []).map(t => `${t.name} (${t.file}) clause ${t.clause} — catches: ${t.catches}`).join('\n') || '(no tests added or changed)'}
</implementer-tests>

<implementer-deviations>
${(work.deviations ?? []).map(d => `clause ${d.clause}: built ${d.built} — ${d.why}`).join('\n') || '(none declared)'}
</implementer-deviations>

${renderCarriedFix(work)}

${GREEN}

Fill \`suite\` with one row per command in verification.md and the exit code you read yourself —
that array is what the run checks, not your prose. Omitting a command is a RED verdict, not a
shorter suite.

Reproduce every evidence clause yourself and give each one a \`clauses\` row. A clause you could not
reach is \`met: false\` with the reason as its evidence, never a missing row.

**Never run git to undo your own edit.** Copy the file first, mutate it, run the test, restore from
your copy. \`git checkout --\`, \`git restore\`, \`git stash\` and \`git reset\` all reset to HEAD, and the
ticket's work is uncommitted — so each of them silently deletes the implementer's changes to every
file you name. This has happened twice, on GTW-1215 and GTW-1183; both times the file was rebuilt
from a captured diff and the run continued, which is luck, not a recovery. Reading git is fine:
\`status\`, \`diff\` and \`log\` change nothing.

Run \`git -C ${REPO} status --porcelain\` and compare it against the claimed file list above. Set
\`reportMatchesTree\` false if the implementer named a file it did not touch or missed one it did,
and say which in your report. An implementer has reported doing nothing while its branch held six
edited files, so this is a real check, not a formality.

Check the two claim lists against the tree, not against the prose:

- Every correction in the CLAUSE AUDIT section must have a row above. Put in
  \`unansweredCorrections\` the clause number of any correction with no row, and of any row claiming
  \`applied\` where the code says otherwise. Open the file and look — an honest \`applied: false\`
  is not a finding, a false \`true\` is.
- Put in \`undeclaredDeviations\` anything the code does differently from what the contract
  **requires** and the implementer did not declare. A deviation it declared is the gate's business,
  not this field's. Files named in the carried-fix section above are not a deviation either: check
  they are in the tree and that the clause they claim to unblock is one of this ticket's, then say
  in your report what you found. Files it changed that it named nowhere still belong in this field.

  A prediction is not a requirement. Clause prose that guesses at an incidental number — a line
  count a change is expected to land on, a count of call sites, a file size — is a guess about an
  outcome, and a guess that misses is a defect in the clause, not in the code. Say so in your
  report and leave the field empty. It belongs there only when the code fails to do something the
  clause asks for. Measured on GTW-1183: a clause predicted a test file would shrink to about 260
  lines, its own mandated signature pushed the file to 323, and a run whose suite was green and
  whose every clause was met went RED on the gap.

A GREEN verdict with either array non-empty is a contradiction; the run reads it as RED.

${HOUSE_RULES}`,
    { model: 'sonnet', label: `verify:${TICKET}#${attempt}`, phase: 'Verify', schema: VERIFY_RESULT })
}

async function runLens(lens, verifyOut, attempt, work) {
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

${lens.key === 'deviations' ? renderDeviations(work) : ''}

${renderCarriedFix(work)}

${HOUSE_RULES}

Run ZERO cargo.

Say NON-COMPLIANT only for a clause you can show is unmet, and give it a \`findings\` row: the
clause number, the symbol the repair agent must open, the file holding it, the line quoted exactly,
and why that line fails that clause. Never a bare line number — code-navigation.md.

The rows ARE the review. A finding described only in \`report\` is a finding nobody fixes, and a
COMPLIANT verdict carrying rows is read as NON-COMPLIANT. \`report\` carries what you opened and
traced, including the clauses you found met.`,
    { model: 'opus', label: `gate:${lens.key}#${attempt}`, phase: 'Gate', agentType: 'design-gate', schema: LENS_RESULT })
}

phase('Verify')
let work = built
let verifyOut = await verify(work, 1)

phase('Gate')
let attempt = 1
// The deviations lens has nothing to judge when none were declared, so it is not spawned.
const lensesFor = (w) => LENSES.filter(l => l.key !== 'deviations' || (w?.deviations?.length ?? 0) > 0)
let activeLenses = lensesFor(work)
let verdicts = await parallel(activeLenses.map(l => () => runLens(l, verifyOut, attempt, work)))

// verifyOut and verdicts are overwritten each round; this is the only record of the earlier ones.
const roundLog = []
const recordRound = () => roundLog.push(
  `round ${attempt}: verify=${verifyOut?.verdict ?? 'MISSING'} `
  + `suite=${(verifyOut?.suite ?? []).map(r => `${r.command}:${r.exit}`).join(' ')} `
  + `lenses=${verdicts.map((v, i) => `${activeLenses[i].key}:${v?.verdict ?? 'MISSING'}`).join(' ')}`
  + (verdicts.flatMap(v => v?.findings ?? []).length
    ? `\n  lens findings this round: ${verdicts.flatMap(v => v?.findings ?? []).map(f => `clause ${f.clause} ${f.symbol}`).join('; ')}`
    : ''))
recordRound()

// Both checks are fail-closed: a verdict is believed only when the rows behind it agree. A GREEN
// with a missing or non-zero suite row, or a COMPLIANT carrying findings, is a disagreement with
// itself and counts as the failing reading.
const green = () => verifyOut?.verdict === 'GREEN'
  && suiteGreen(verifyOut.suite)
  && (verifyOut.unansweredCorrections?.length ?? 0) === 0
  && (verifyOut.undeclaredDeviations?.length ?? 0) === 0
const compliant = (v) => v?.verdict === 'COMPLIANT' && (v.findings?.length ?? 0) === 0
const allPass = () => green() && verdicts.every(compliant)
const reviewersHeardFrom = () => (verifyOut ? 1 : 0) + verdicts.filter(Boolean).length

while (!allPass() && attempt < 5) {
  if (reviewersHeardFrom() === 0) {
    throw new Error(`${TICKET}: no reviewer reported (round ${attempt}). Infrastructure failure — resume.`)
  }
  attempt++
  phase('Fix')

  const failedCommands = (verifyOut?.suite ?? []).filter(r => r.exit !== 0)
  const unmetClauses = (verifyOut?.clauses ?? []).filter(c => !c.met)
  const contractGaps = [
    (verifyOut?.unansweredCorrections ?? []).length
      ? `audit corrections with no honest answer: ${verifyOut.unansweredCorrections.join(', ')} — apply each one, or say plainly in \`corrections\` that it needed nothing and why.`
      : null,
    (verifyOut?.undeclaredDeviations ?? []).length
      ? `undeclared deviations:\n${verifyOut.undeclaredDeviations.map(d => `- clause ${d.clause}: ${d.differs}`).join('\n')}`
      : null,
  ].filter(Boolean).join('\n\n')

  const problems = [
    green() ? null : verifyOut
      ? `<verify-report verdict="${verifyOut.verdict}"${verifyOut.reportMatchesTree ? '' : ' files-claimed-do-not-match-the-tree="true"'}>
failing suite commands: ${failedCommands.map(r => `${r.command} (exit ${r.exit})`).join(', ') || '(none reported — the suite rows are missing or incomplete)'}
unmet clauses: ${unmetClauses.map(c => `${c.clause} — ${c.evidence}`).join('\n') || '(none reported)'}

${verifyOut.report}
</verify-report>`
      : `<verify-report>\n(died — nothing to act on)\n</verify-report>`,
    contractGaps ? `<contract-gaps>\n${contractGaps}\n</contract-gaps>` : null,
    ...verdicts.map((v, i) => compliant(v) ? null : renderFindings(v, activeLenses[i].key)),
  ].filter(Boolean).join('\n\n')

  log(`${TICKET}: round ${attempt} — repairing`)

  work = await agent(`Repair ${TICKET} in ${REPO}.

<ticket id="${TICKET}">
${contract}
</ticket>

## WHAT BLOCKED IT
${problems}

${renderCarriedFix(work)}

Fix at root. Do NOT weaken tests and do NOT edit the gate.

Files dirty in the tree that are not this ticket's work stay exactly as they are — do not stage
them and do not revert them. A repair round has discarded an unrelated edit before.

${HOUSE_RULES}
${GREEN}

Re-run the suite. Do NOT commit.

Report on the same terms as the build: \`filesChanged\` matching \`git status --porcelain\`, one
\`clauses\` row per clause, one \`corrections\` row per audit correction, a \`tests\` row naming the
mutation each test turns red, \`deviations\` empty unless you truly could not build a clause as
written, dirty files that are not this ticket's in \`foreignDirtyFiles\`, and anything ticket-worthy
outside the contract in \`outOfScope\`.

\`carriedFix\` carries forward. Any carried fix declared above is still in the tree, so repeat it in
your own \`carriedFix\` field, adding any file this round touched for the same defect. Drop it and
land commits those files under this ticket instead of the fix's own.`,
    { model: 'opus', label: `fix:${TICKET}#${attempt}`, phase: 'Fix', schema: WORK_RESULT })

  if (!work) throw new Error(`fix agent died on ${TICKET} round ${attempt}`)

  phase('Verify')
  verifyOut = await verify(work, attempt)
  phase('Gate')
  activeLenses = lensesFor(work)
  verdicts = await parallel(activeLenses.map(l => () => runLens(l, verifyOut, attempt, work)))
  recordRound()
}

if (!allPass()) {
  return {
    ticket: TICKET, landed: false, reason: `still blocked after ${attempt} rounds`,
    verify: verifyOut?.report ?? null,
    verifySuite: verifyOut?.suite ?? [],
    unmetClauses: (verifyOut?.clauses ?? []).filter(c => !c.met),
    gate: verdicts.map((v, i) => ({
      lens: activeLenses[i].key,
      verdict: v?.verdict ?? 'MISSING',
      findings: v?.findings ?? [],
      report: v?.report ?? null,
    })),
  }
}

phase('Docs-sync')

const docs = await agent(`Docs-sync posture for ${TICKET} in ${REPO}.

If the change touched behaviour described in docs/, verify each claim against the code and fix the
drift. Code is authority for what exists; docs/ remain authority for design intent.

Return a \`claims\` row for every written claim you checked, including the ones that already matched
— those rows are the only evidence the sweep happened, and "nothing drifted" with no rows is
indistinguishable from not looking. \`scope\` names the docs you judged in range, including any you
opened and found untouched.

Where the code and a doc disagree about design intent, do NOT pick a side: give it a \`conflicts\`
row and leave both alone (\`.claude/rules/design-fidelity.md\` rule 4).

Files dirty in the tree that are not this ticket's work stay exactly as they are.

Every word you write into docs/ follows \`.claude/rules/plain-language.md\` and the doc-register
section of the \`/docs-sync\` skill: no ticket id, no clause number, no plan or intention, and no
coined figure of speech where a literal description works. That applies to text you edit as well as
text you add — if a claim you are already fixing carries a GTW id or a clause number, strip it in
the same pass and say so in its \`claims\` row.

Do NOT commit — /land owns the commit.`,
  { model: 'opus', label: `docs-sync:${TICKET}`, phase: 'Docs-sync', schema: DOCS_RESULT })

phase('Land')

// The carried fix is committed under its own ticket, so that ticket has to exist before land runs.
// Only a project-manager step can write to Linear (HOUSE_RULES rule 7), and linear-discipline.md
// rule 4 puts the ticket before the fix.
const CARRIED_TICKET_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['filed', 'ticket', 'title', 'report'],
  properties: {
    filed: { type: 'boolean', description: 'Did the ticket get created. False with a reason in report is honest; a made-up id is not.' },
    ticket: { type: 'string', description: 'The id the board returned, as GTW-n. Empty if nothing was filed.' },
    title: { type: 'string', description: 'The title the board shows for it.' },
    report: { type: 'string', description: 'What you posted, what the board said back, and the relation you set.' },
  },
}

const carriedFixDeclared = (work.carriedFix?.files?.length ?? 0) > 0

const carriedTicket = carriedFixDeclared
  ? await agent(`File the carried fix's ticket for ${TICKET} in project GDTF.

The build of ${TICKET} fixed a defect that blocked its clause ${work.carriedFix.clause}, under the
carried fix section of \`.claude/rules/design-fidelity.md\`. The fix is written and uncommitted in
${REPO} on ${BRANCH}, which is the one case \`linear-discipline.md\` rule 4 allows. The land step
commits those files on their own under the id you return, so it cannot run until this ticket
exists.

File one ticket and nothing else:

- title: ${work.carriedFix.title}
- label: Bug
- state: In Progress
- relation: blocks ${TICKET}
- description: the text below, with no source line above it. Every description is agent-written.

<description>
${work.carriedFix.defect}

Files: ${work.carriedFix.files.join(', ')}

Fixed by the build of ${TICKET}, which could not show its clause ${work.carriedFix.clause} green
until this was fixed. The fix lands as its own commit under this ticket.
</description>

Return the id the board gave it in \`ticket\`. You judge nothing: the wording above was written by
the build, so post it as it stands. Do not move ${TICKET}, and do not close anything.`,
    { model: 'sonnet', label: `file-carried:${TICKET}`, phase: 'Land', agentType: 'project-manager', schema: CARRIED_TICKET_RESULT })
  : null

const carriedTicketId = /^GTW-\d+$/i.test(carriedTicket?.ticket ?? '') ? carriedTicket.ticket : null

if (carriedFixDeclared && !carriedTicketId) {
  return {
    ticket: TICKET, landed: false,
    reason: 'the carried fix has no ticket of its own, so land cannot commit it',
    carriedFix: work.carriedFix,
    fileCarried: carriedTicket?.report ?? '(the step that files it died)',
  }
}

if (carriedTicketId) {
  log(`${TICKET}: carried fix filed as ${carriedTicketId}, for the defect blocking clause ${work.carriedFix.clause}`)
}

const landed = await agent(`Land ${TICKET} following the /land skill.

Repo: ${REPO} — no worktree; the work is on ${BRANCH} in the main tree.

Verify is ${verifyOut.verdict} and all ${activeLenses.length} gate lenses are COMPLIANT after ${attempt} round(s).
The run does not reach you otherwise. You re-run the suite yourself on the tree you commit; their
reports are not yours to read, and the summarize step re-derives anything anyone needs from them.

## FILES THAT ARE NOT THIS TICKET'S
${(work.foreignDirtyFiles ?? []).length ? work.foreignDirtyFiles.map(f => `- ${f}`).join('\n') : '(none reported)'}

Never stage or revert those. They belong to someone else and a repair round has discarded one before.

## THE CARRIED FIX
${carriedTicketId ? `The build fixed a defect that blocked clause ${work.carriedFix.clause} of ${TICKET}, and it is filed as ${carriedTicketId}. These are its files:

${work.carriedFix.files.map(f => `- ${f}`).join('\n')}

Stage exactly those paths and commit them FIRST, on their own, with the subject
\`Area: summary (${carriedTicketId})\` and a body saying what was broken. ${TICKET}'s tree stands on
that fix, so the fix is the earlier of the two commits. Then stage ${TICKET}'s own files and commit
them as step 3 says. Then rebase and fast-forward, which carries both commits to develop.

Report that commit's sha, subject and ticket in \`carriedFixCommit\`. It is expected, so it does not
also go in \`carriedCommits\`.` : '(none. The build carried no fix, so leave every field of `carriedFixCommit` empty.)'}

Run every command in the foreground and read its exit code in the same turn. Never start the suite
in the background and end your turn waiting on it — that is how a previous attempt at this step died
without committing. There is no monitor coming to report your exit codes.

${GREEN}

## What you own

Steps 1 to 5 of .claude/skills/land/SKILL.md — the git half. Read that file and follow it; it is
the only step list. Do not do steps 6 or 7: the close phase owns moving the ticket to Done and
holds the Linear tools you do not, and the confirm phase owns reporting the pushed range and
deleting .claude/.gate-pass. Leave the gate-pass file on disk so that phase can compare it against
your report.

Not doing 6 and 7 is the design, not a rule you broke. Do not report their absence as a finding.

Four things this workflow pins on top of that file:

- Re-run the full green suite before step 1. Docs-sync ran after the gate, so the tree you are
  about to commit is not the tree the verify report certified. The build cache makes it cheap when
  nothing changed.
- Files dirty in the tree that are not this ticket's work are left alone — not staged, not
  reverted. They are listed above. IF, AND ONLY IF, required, you may temporarily move the files
  out of the tree in order to land. IF YOU DO, the files must be restored before you finish.
- If \`--ff-only\` fails, STOP and report. Never fall back to a merge commit. If the rebase conflicts,
  resolve it, stage the resolved files, and continue with \`git rebase --continue\` — never the
  commit subcommand, because the pre-commit hook blocks every commit on develop.
- Finish on develop. Never \`--no-verify\`. Report what git said, and do not claim landing is proven —
  a separate confirm step checks origin/develop.

## Reporting

Every field is something git printed this run, not something you remember.

- \`carriedCommits\`: run \`git -C ${REPO} log --oneline origin/develop..${BRANCH}\` BEFORE the
  fast-forward and list every commit whose subject does not name ${TICKET}, apart from the carried
  fix commit you made yourself. They reach develop with this land whether they belong to it or not.
  Report them. Do not delete them and do not rewrite history.
- \`carriedFixCommit\`: the sha, subject and ticket of the commit you made for the carried fix, or
  three empty strings when the section above said there was none. Read the sha from the same
  \`git log\` you ran for \`carriedCommits\`, AFTER the rebase, because the rebase rewrites it. The
  confirm step re-finds that commit on origin/develop by its subject, so the subject you report is
  the subject you wrote.
- \`rebase\` and \`conflictedFiles\`: say what the rebase did. If you resolved a conflict, name every
  file you resolved — a quiet revert of someone else's work hides exactly there.
- \`developBefore\` and \`developAfter\`: the two shas around the fast-forward.
- \`gatePass\`: the fingerprint, scope and head you wrote into the file.
- \`filesLeftUnstaged\`: the dirty paths you deliberately did not stage.
- \`branchDeleted\`: whether the local feature branch is gone.

You report no findings, risks or judgements about the code — there is no field for them. Your
context still holds the rounds of this run that failed, so you cannot tell a live defect from one
an engineer has since fixed. A later step gathers its own evidence and decides. Land, and report
what git printed.`,
  { model: 'sonnet', label: `land:${TICKET}`, phase: 'Land', schema: LAND_RESULT })

const confirm = await agent(`Confirm landing of ${TICKET}.

Main repo: ${REPO}

1. \`git -C ${REPO} fetch origin develop\`
2. Find the commit on origin/develop whose subject contains ${TICKET} (most recent if several).
3. \`git -C ${REPO} merge-base --is-ancestor <sha> origin/develop\`

Return that sha in \`landedSha\` only if step 3 exits 0. Otherwise return the empty string. Return
its subject line too, so the sha can be checked against the ticket rather than assumed, and
origin/develop's head in \`developSha\`.

4. ${carriedTicketId
  ? `This land carried a fix for ${carriedTicketId}, committed on the same branch under its own
subject. Find the commit on origin/develop whose subject contains ${carriedTicketId} (most recent if
several), run \`git -C ${REPO} merge-base --is-ancestor <sha> origin/develop\`, and return that sha in
\`carriedFixSha\`, with its subject line in \`carriedFixSubject\`, only if it exits 0. The land step
reads its sha on the branch, and a rebase rewrites shas, so yours is the sha that is on develop.
Report what you find whatever the land step said, because it may have made that commit and left its
own report of it empty. That commit is expected. It does not also go in \`extraCommits\`.`
  : '(this run carried no fix, so leave `carriedFixSha` and `carriedFixSubject` empty)'}

5. \`git -C ${REPO} log --oneline ${landed?.developBefore || 'origin/develop@{1}'}..origin/develop\`

Every commit in that range whose subject does not name ${TICKET}${carriedTicketId ? ` or ${carriedTicketId}` : ''} goes in
\`extraCommits\`. You are the only step that fetches origin, so this is the only place the question
gets asked. Report them; never try to remove one.

6. You own the rest of step 7 of .claude/skills/land/SKILL.md: once step 3 has exited 0, compare
\`.claude/.gate-pass\` against what the land step reported and then delete the file. The land step
must leave it on disk for exactly this comparison, and a gate-pass naming a branch that no longer
exists blocks the next commit on the next feature branch. If step 3 did not exit 0, leave the file
alone — the land is unproven and the file is still the record of what was staged.

Change nothing else.`,
  { model: 'sonnet', label: `confirm-land:${TICKET}`, phase: 'Land', schema: CONFIRM_RESULT })

const landedSha = /^[0-9a-f]{7,40}$/i.test(confirm?.landedSha ?? '') ? confirm.landedSha : null

// Contradictions inside one report are read as the failing side, never the convenient one.
if (landed?.committed && !landed.commitSha) {
  log(`${TICKET}: land reported committed with no sha — treating the land as unproven`)
}

// The two git commands print shas of different lengths, so they are compared by prefix.
const sameCommit = (a, b) => !!a && !!b && (a.startsWith(b) || b.startsWith(a))

// The id this run filed decides whether there is a carried fix, never the land step's word for it.
// Read from the report alone, a sha land names on a run that filed nothing would be pulled out of
// `carried` and reported under a ticket nobody filed. `carried` is where that rider must show up.
//
// The sha comes from confirm, which read origin/develop after the rebase, the same way `landedSha`
// does. Land reads its sha on the branch, and a rebase that replays the commit rewrites it.
//
// Either report alone is enough to say the commit exists. Confirm is told to keep it out of
// `extraCommits`, so a land step that made the commit and left its own report of it empty would
// otherwise leave a commit on develop that no field of this run names.
const confirmedCarriedSha = /^[0-9a-f]{7,40}$/i.test(confirm?.carriedFixSha ?? '') ? confirm.carriedFixSha : null
const landCarriedSha = landed?.carriedFixCommit?.sha || null
const carriedFixCommit = carriedTicketId && (confirmedCarriedSha || landCarriedSha)
  ? {
    sha: confirmedCarriedSha || landCarriedSha,
    subject: landed?.carriedFixCommit?.subject || confirm?.carriedFixSubject || '',
    ticket: landed?.carriedFixCommit?.ticket || carriedTicketId,
  }
  : null
if (confirmedCarriedSha && landCarriedSha && !sameCommit(confirmedCarriedSha, landCarriedSha)) {
  log(`${TICKET}: the rebase rewrote the carried fix commit. ${landCarriedSha} on the branch is ${confirmedCarriedSha} on develop`)
}
if (carriedFixCommit && !confirmedCarriedSha) {
  log(`${TICKET}: confirm found no commit naming ${carriedTicketId} on origin/develop. The reported sha is the one land read on the branch`)
}
if (carriedFixCommit && !landCarriedSha) {
  log(`${TICKET}: land reported no commit for ${carriedTicketId} and confirm found ${confirmedCarriedSha} on develop under it`)
}

// Where the report and the filed id disagree, the run says so instead of dropping one side.
if (!carriedTicketId && landCarriedSha) {
  log(`${TICKET}: land named ${landCarriedSha} a carried fix and this run filed none. It stays a rider`)
}
if (carriedTicketId && !landCarriedSha && !confirmedCarriedSha) {
  log(`${TICKET}: ${carriedTicketId} was filed for the carried fix and no commit under it reached develop`)
}

// carriedCommits and extraCommits hold what nobody expected. The carried fix commit was expected,
// so it is reported under its own name and never as a rider.
//
// The subject decides which commit that is, because the run told land to write
// `Area: summary (<id>)` and a rebase cannot change what it says. The sha is checked too and is the
// weaker test: land reads its sha on the branch, confirm reads the rider shas off develop, so a
// rebase that replays the commit leaves only the subject matching. The id is validated as
// `GTW-<digits>`, so it holds no regex character, and `\b` keeps GTW-136 from matching GTW-1368.
//
// A sha land named on a run that filed no ticket is not the carried fix, so it stays in `carried`.
const namesCarriedTicket = carriedTicketId
  ? new RegExp(`\\b${carriedTicketId}\\b`, 'i')
  : null
const isTheCarriedFix = c => !!carriedFixCommit && (
  namesCarriedTicket.test(c.subject ?? '')
  || sameCommit(c.sha, carriedFixCommit.sha)
  || sameCommit(c.sha, landCarriedSha))
const carried = [...(landed?.carriedCommits ?? []), ...(confirm?.extraCommits ?? [])]
  .filter(c => !isTheCarriedFix(c))
if (carriedFixCommit) {
  log(`${TICKET}: carried fix landed as ${carriedFixCommit.sha} under ${carriedFixCommit.ticket || carriedTicketId}`)
}
if (carried.length) {
  log(`${TICKET}: ${carried.length} commit(s) reached develop that do not name this ticket — ${carried.map(c => c.sha).join(', ')}`)
}
if (landed?.rebase === 'conflicts-resolved' && landed.conflictedFiles?.length) {
  log(`${TICKET}: rebase resolved conflicts in ${landed.conflictedFiles.join(', ')} — check none of it was someone else's work`)
}

// Nothing upstream is trusted here: the land step's context still holds the rounds that failed.
const SUMMARY_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['summary', 'findings', 'rejected', 'followUps', 'nextTickNotes'],
  properties: {
    summary: { type: 'string', description: 'What the orchestrator tells the user. Written for someone who has read none of the run. Says nothing about the ticket\'s status, comments or labels: the close step has not run yet, so you cannot know them.' },
    findings: {
      type: 'array', description: 'Things worth a look that you reproduced on the landed tree yourself. Never the ticket\'s board state: the close step runs after you, so an unmoved status or a missing evidence comment is the pipeline working, not a finding.',
      items: {
        type: 'object', additionalProperties: false, required: ['kind', 'detail', 'evidence'],
        properties: {
          kind: { type: 'string', enum: ['out-of-scope', 'process-violation', 'risk'] },
          detail: { type: 'string' },
          evidence: { type: 'string', description: 'The command you ran and what it printed.' },
        },
      },
    },
    rejected: {
      type: 'array', description: 'Claims you checked and did not keep. Writing each one down is what stops a silent discard; nothing downstream reads them.',
      items: {
        type: 'object', additionalProperties: false, required: ['claim', 'why'],
        properties: {
          claim: { type: 'string' },
          why: { type: 'string', description: 'What you ran and what it showed.' },
        },
      },
    },
    followUps: {
      type: 'array', description: 'Findings that should become tickets. Only ones you reproduced.',
      items: {
        type: 'object', additionalProperties: false, required: ['title', 'label', 'why', 'evidence'],
        properties: {
          title: { type: 'string', description: 'The ticket title, as it would be filed.' },
          label: { type: 'string', description: 'One of the kind labels in linear-discipline.md.' },
          why: { type: 'string', description: 'What is wrong and where, by symbol or quoted text.' },
          evidence: { type: 'string', description: 'The run that shows it, with its numbers.' },
        },
      },
    },
    nextTickNotes: { type: 'string', description: 'What the next tick needs that it cannot read off the tree. Empty if nothing. Not the ticket\'s status or its comments; the close step owns those and runs after you.' },
  },
}

const summarized = landedSha
  ? await agent(`Write the report for ${TICKET}. You own every judged field; gather your own evidence.

Main repo: ${REPO}. Landed commit: ${landedSha}.

The steps below reported to you, but their context also holds every round of this run that failed —
red suites, gate lenses that failed, defects an engineer then fixed. So anything reading like a
defect may describe a state that no longer exists. Nothing reaches your output on their word.

**Check the tree before you write a finding.** Open the file, grep the line, re-run the test, read
\`git -C ${REPO} show ${landedSha} -- <path>\`. Three ways a claim dies, all measured:

1. **Its fix is in the landed diff.** A docs line called stale that this commit corrected.
2. **A later round fixed it.** The history below gives every round in order. A defect named in a
   failing round, then a GREEN verify and COMPLIANT lenses, is history unless it still reproduces.
3. **It will not reproduce.** Re-measure; report your number, not the one you were handed. A flaky
   test gets run enough times to get a rate.

Everything you checked and dropped goes in \`rejected\` with what you ran. Anything that survives
and deserves a ticket goes in \`followUps\` — those become real tickets, so each needs a title, a
symbol or quoted text, and the run behind it.

**The board is not yours to report.** The close step runs AFTER you, so Linear still shows this
ticket as it was before the run finished: In Progress, with no evidence comment on it. That is the
close step's job, not a defect. Never write the ticket's status, its comments or its labels into any
field. Those come from the close step's own return, which happens after you have finished, and a
claim from you about them is wrong by construction. This step used to report a process violation
against itself four times in one day for exactly this reason.

<land-result>
${JSON.stringify(landed, null, 2)}
</land-result>

<confirm-result>
${JSON.stringify(confirm, null, 2)}
</confirm-result>

<carried-fix>
${carriedFixCommit
  ? `${carriedFixCommit.sha} ${carriedFixCommit.subject}\nticket: ${carriedFixCommit.ticket || carriedTicketId}\nclause of ${TICKET} it unblocked: ${work.carriedFix?.clause}\nfiles: ${(work.carriedFix?.files ?? []).join(', ')}\n\nThis commit was made on purpose, under the carried fix section of .claude/rules/design-fidelity.md.\nIt is not a rider and not a process violation.`
  : '(none. This run carried no fix.)'}
</carried-fix>

<commits-that-do-not-name-this-ticket>
${JSON.stringify(carried, null, 2)}
</commits-that-do-not-name-this-ticket>

<run-history rounds="${attempt}">
${roundLog.join('\n')}
</run-history>

<final-verify verdict="${verifyOut?.verdict}">
${verifyOut?.report ?? ''}
</final-verify>

<final-gate>
${verdicts.map((v, i) => `<lens name="${activeLenses[i].key}" verdict="${v?.verdict ?? 'MISSING'}"/>`).join('\n')}
</final-gate>

<audit-corrections>
${JSON.stringify(audit?.corrections ?? [], null, 2)}
</audit-corrections>

<implementer-out-of-scope>
${JSON.stringify(work?.outOfScope ?? [], null, 2)}
</implementer-out-of-scope>

<foreign-dirty-files>
${JSON.stringify(work?.foreignDirtyFiles ?? [], null, 2)}
</foreign-dirty-files>

<docs-sync>
${JSON.stringify({ changed: docs?.filesChanged ?? [], conflicts: docs?.conflicts ?? [] }, null, 2)}
</docs-sync>`,
    { model: 'sonnet', label: `summarize:${TICKET}`, phase: 'Land', agentType: 'source-control',
      schema: SUMMARY_RESULT })
  : null

if (summarized?.rejected?.length) {
  log(`${TICKET}: ${summarized.rejected.length} claim(s) did not survive re-checking against ${landedSha}`)
}

const closed = landedSha
  ? await agent(`Close ${TICKET} in Linear with evidence.

Move to In Review then Done. Comment BEFORE the status change — an archived issue rejects a
comment and there is no un-archive. The comment opens with the source line \`**[build-ticket / land]**\`
and nothing above it, per \`.claude/rules/linear-discipline.md\`, then carries: landing sha ${landedSha},
the suite result, and which evidence clauses were satisfied.

Check the ticket's children before AND after the close. Cascades run both ways and are not
reliable, so report in \`boardEffects\` anything that moved besides this ticket.

Return the board's own id for the comment you post in \`commentId\`, and the ticket's labels after
the close in \`labelsAfter\` — a process flag left on, like Needs User Input or Needs Splitting, is
visible nowhere else.

You judge nothing here. The summary below was written by a step that re-checked the tree; post it
as it stands rather than rewriting it, and do not add a defect of your own.

Where the carried fix section below names a commit, that fix's own ticket landed with this push.
Close it on the same terms as this one: post a comment first, opening with the source line
\`**[build-ticket / land]**\` and carrying that commit's sha, then move it to Done. Record both in
\`boardEffects\`. \`.claude/rules/linear-discipline.md\` rule 2 leaves no landed ticket open.

<summary>
${summarized?.summary ?? '(no summary — say so rather than writing one)'}
</summary>

<green-suite>
${(landed?.suite ?? []).map(r => `${r.command} — exit ${r.exit}`).join('\n') || '(no suite rows reported)'}
</green-suite>

<gate rounds="${attempt}">
verify ${verifyOut.verdict}; ${verdicts.map((v, i) => `${activeLenses[i].key} ${v?.verdict ?? 'MISSING'}`).join(', ')}
</gate>

<clauses-satisfied>
${(verifyOut.clauses ?? []).map(c => `${c.clause}: ${c.met ? 'met' : 'NOT met'}`).join('\n') || '(none reported)'}
</clauses-satisfied>

<findings-worth-recording>
${(summarized?.findings ?? []).map(f => `- [${f.kind}] ${f.detail}`).join('\n') || '(none)'}
</findings-worth-recording>

<follow-ups-the-orchestrator-will-file>
${(summarized?.followUps ?? []).map(f => `- ${f.title} (${f.label}) — ${f.why}`).join('\n') || '(none)'}
</follow-ups-the-orchestrator-will-file>

<carried-fix>
${carriedFixCommit
  ? `${carriedFixCommit.sha} ${carriedFixCommit.subject}\nticket: ${carriedFixCommit.ticket || carriedTicketId}\nit unblocked clause ${work.carriedFix?.clause} of ${TICKET}`
  : '(none)'}
</carried-fix>

<commits-that-do-not-name-this-ticket>
${carried.map(c => `${c.sha} ${c.subject}`).join('\n') || '(none)'}
</commits-that-do-not-name-this-ticket>`,
    { model: 'sonnet', label: `close:${TICKET}`, phase: 'Land', agentType: 'project-manager', schema: CLOSE_RESULT })
  : null

return {
  // Summarize first: a long result is truncated in the notification, and these are the fields the
  // orchestrator acts on. The bulky prose sits at the end where losing it costs nothing.
  ticket: TICKET,
  landed: !!landedSha,
  landedSha,
  // The carried fix's own commit, next to the ticket's. Both landed in this push, under different
  // ticket ids, and this is where the record says which is which.
  carriedFix: carriedFixCommit
    ? {
      sha: carriedFixCommit.sha,
      subject: carriedFixCommit.subject,
      ticket: carriedFixCommit.ticket || carriedTicketId,
      clause: work.carriedFix?.clause ?? null,
      files: work.carriedFix?.files ?? [],
    }
    : null,
  rounds: attempt,
  summary: summarized?.summary ?? null,
  findings: summarized?.findings ?? [],
  followUps: summarized?.followUps ?? [],
  nextTickNotes: summarized?.nextTickNotes ?? null,
  ticketState: closed?.ticketState ?? 'not closed — landing was not confirmed',
  boardEffects: closed?.boardEffects ?? [],
  labelsAfter: closed?.labelsAfter ?? [],
  commentId: closed?.commentId ?? null,
  carriedCommits: carried,
  docsConflicts: docs?.conflicts ?? [],
  outOfScope: work.outOfScope ?? [],
  foreignDirtyFiles: work.foreignDirtyFiles ?? [],
  roundLog,
  suite: landed?.suite ?? [],
  verifySuite: verifyOut.suite ?? [],
  reportMatchedTree: verifyOut.reportMatchesTree,
  // Clause numbers only. The rewritten clauses were posted to the ticket by the open step, so
  // repeating them here is bulk that pushes the fields above out of a truncated notification.
  correctedClauses: (audit.corrections ?? []).map(c => c.clause),
  correctionsAnswered: (work.corrections ?? []).map(c => `${c.clause} applied=${c.applied}`),
  filesStaged: landed?.filesStaged ?? [],
  filesLeftUnstaged: landed?.filesLeftUnstaged ?? [],
  docsChanged: docs?.filesChanged ?? [],
  rebase: landed?.rebase ?? null,
  conflictedFiles: landed?.conflictedFiles ?? [],
  developBefore: landed?.developBefore ?? null,
  developAfter: landed?.developAfter ?? null,
  gatePass: landed?.gatePass ?? null,
  branchDeleted: landed?.branchDeleted ?? null,
  verify: verifyOut.report,
  close: closed?.report ?? null,
}

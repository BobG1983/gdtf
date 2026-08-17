export const meta = {
  name: 'split-ticket',
  description: 'Split one epic into buildable children: propose, edit, revise, vote to consensus, map clause ownership, write each ticket, audit until clean, file with edges.',
  phases: [
    { title: 'Read', detail: 'the whole parent ticket and its canon' },
    { title: 'Propose', detail: 'N independent splits, each from a different principle' },
    { title: 'Edit', detail: 'one editor per proposal' },
    { title: 'Revise', detail: 'each proposal rewritten from its editor notes' },
    { title: 'Vote', detail: '4 lenses score every proposal' },
    { title: 'Argue', detail: 'lenses answer each other and re-vote, capped' },
    { title: 'Settle', detail: "fold the panel's findings into the winning proposal" },
    { title: 'Map', detail: 'every parent clause against the children that own it' },
    { title: 'Write', detail: 'one agent per child writes its whole ticket' },
    { title: 'Audit', detail: 'audit and fix each ticket until no new finding' },
    { title: 'File', detail: 'one agent per child, then the edges and the parent' },
  ],
}

// args: {
//   ticket:    "GTW-1175"                     required
//   proposals: 3                              how many splits to propose (usually 3-10)
//   scratch:   "/abs/path"                    where stage files are written (usually /tmp/private/...)
//   startFrom: "propose" | "vote" | "map"     that stage and everything after it
//   winner:    "/abs/path/proposal-2.md"      required when startFrom is "map"
//   proposalPaths: ["/abs/a.md", …]           required when startFrom is "vote"
// }
//
// Re-entry exists because this workflow is allowed to stop without an answer. A
// vote that never reaches consensus returns the disagreement instead of picking
// for you; you settle it and re-enter at "map" with the file you chose.
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { throw new Error(`args was an unparseable string: ${A}`) }
}
const TICKET = A?.ticket
if (!TICKET) throw new Error(`args must supply { ticket }; got ${JSON.stringify(args)}`)

const PROPOSAL_COUNT = A?.proposals ?? 3
const MAX_VOTE_ROUNDS = 5
const MAX_AUDIT_ROUNDS = 4
const MAX_MAP_ROUNDS = 5
const START_FROM = A?.startFrom ?? 'propose'
if (!['propose', 'vote', 'map'].includes(START_FROM)) throw new Error(`startFrom must be propose, vote or map; got ${START_FROM}`)
if (START_FROM === 'map' && !A?.winner) throw new Error('startFrom "map" needs args.winner — the proposal file to build from')
if (START_FROM === 'vote' && !(A?.proposalPaths || []).length) throw new Error('startFrom "vote" needs args.proposalPaths')

const REPO = (typeof process !== 'undefined' && process.cwd && process.cwd()) || '.'
const JOB_TMP = (typeof process !== 'undefined' && process.env && process.env.CLAUDE_JOB_DIR)
  ? `${process.env.CLAUDE_JOB_DIR}/tmp`
  : '/tmp'
const SCRATCH = A?.scratch ?? `${JOB_TMP}/split-${TICKET}`

const HOUSE = `House rules, all under \`${REPO}/.claude/rules/\` — read the ones you touch:
- \`clause-writing.md\` — a clause says what changes, where, and what goes red if it is wrong.
- \`plain-language.md\` — plain wording, shortest text that carries the information.
- \`linear-discipline.md\` — cite a symbol or quote the text, never a bare line number. Feature / Editor / Improvement that adds a player or author verb must include the MCP surface block; refuse a draft that lacks it.
- \`design-fidelity.md\` — build exactly what is specified; never narrow a requirement to make it fit.
- \`verification.md\` — the one definition of green.`

// --- Schemas ---------------------------------------------------------------

const PARENT = {
  type: 'object', additionalProperties: false,
  required: ['title', 'description', 'clauses', 'comments', 'labels', 'canonPaths', 'constraints', 'outOfScope'],
  properties: {
    title: { type: 'string' },
    description: { type: 'string', description: 'The full description verbatim, every section, never a summary.' },
    clauses: {
      type: 'array', description: 'Every Done-when clause, verbatim, numbered as the ticket numbers them.',
      items: {
        type: 'object', additionalProperties: false, required: ['n', 'text'],
        properties: { n: { type: 'integer' }, text: { type: 'string', description: 'Verbatim.' } },
      },
    },
    comments: {
      type: 'array', description: 'Every comment, oldest first, verbatim.',
      items: {
        type: 'object', additionalProperties: false, required: ['source', 'body'],
        properties: {
          source: { type: 'string', description: 'The `**[name]**` source line without brackets, or exactly "owner" when the comment has none.' },
          body: { type: 'string' },
        },
      },
    },
    labels: { type: 'array', items: { type: 'string' } },
    canonPaths: { type: 'array', items: { type: 'string' }, description: 'Repo-relative docs the ticket names as canon.' },
    constraints: { type: 'array', items: { type: 'string' }, description: 'Ordering or sequencing constraints stated anywhere in the ticket or its comments.' },
    outOfScope: { type: 'array', items: { type: 'string' }, description: 'What the ticket explicitly excludes.' },
  },
}

const CHILD = {
  type: 'object', additionalProperties: false,
  required: ['key', 'title', 'delivers', 'greenBecause', 'ownsClauses', 'dependsOn'],
  properties: {
    key: { type: 'string', description: 'Short stable slug, unique within this proposal.' },
    title: { type: 'string' },
    delivers: { type: 'string', description: 'What works after this lands that did not before.' },
    greenBecause: { type: 'string', description: 'Why the tree is green with this landed and nothing after it built. Name what would otherwise be half-migrated, disabled or unreachable. A vague answer here is the tell for a bad split.' },
    ownsClauses: {
      type: 'array',
      description: 'Parent clauses this child owns. A clause may be owned in part; say which part.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'portion'],
        properties: {
          clause: { type: 'integer' },
          portion: { type: 'string', description: 'Exactly "whole", or the part of the clause this child owns, in the clause\'s own words.' },
        },
      },
    },
    dependsOn: { type: 'array', items: { type: 'string' }, description: 'Keys of children that must land first.' },
  },
}

const PROPOSAL = {
  type: 'object', additionalProperties: false,
  required: ['id', 'path', 'angle', 'children', 'risks'],
  properties: {
    id: { type: 'string' },
    path: { type: 'string', description: 'Absolute path of the file written.' },
    angle: { type: 'string', description: 'One sentence: the principle this split is organised around.' },
    children: { type: 'array', items: CHILD },
    risks: { type: 'array', items: { type: 'string' } },
  },
}

const EDIT_NOTES = {
  type: 'object', additionalProperties: false,
  required: ['id', 'keep', 'fix'],
  properties: {
    id: { type: 'string' },
    keep: { type: 'string', description: 'What this proposal gets right that a rewrite must not lose.' },
    fix: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['child', 'change', 'why'],
        properties: {
          child: { type: 'string', description: 'The child key, or "split" for a change to the shape itself.' },
          change: { type: 'string' },
          why: { type: 'string' },
        },
      },
    },
  },
}

const LENSES = [
  { key: 'green', question: 'Does every child land GREEN on its own with no later child built? A child that leaves a half-migrated tree, a disabled or deleted test, or a feature nothing reaches fails this lens.' },
  { key: 'order', question: 'Does any child need something a later child produces? Walk the stated dependency edges and the ones the proposal leaves implied.' },
  { key: 'coverage', question: 'Does the union of children equal the parent exactly? Name any parent clause owned by nobody, any part owned twice, and any child doing work the parent never asked for.' },
  { key: 'size', question: 'Is each child one sitting? Name any child that is really two, and any pair that should be one.' },
]

const VOTE = {
  type: 'object', additionalProperties: false,
  required: ['lens', 'ranking', 'best', 'blocking', 'moved', 'counterArguments'],
  properties: {
    lens: { type: 'string' },
    ranking: {
      type: 'array', description: 'Every proposal id, best first.',
      items: {
        type: 'object', additionalProperties: false, required: ['id', 'score', 'why', 'changeMyMindBy'],
        properties: {
          id: { type: 'string' },
          score: { type: 'integer', description: '0-10 against this lens only.' },
          why: { type: 'string' },
          changeMyMindBy: {
            type: 'string',
            description: 'Your price on this proposal. For the one you picked: what would make you drop it. For the others: what would make you pick it. Name a change to the proposal, concrete enough that someone could make it and you would have to move. "Nothing" is an answer, but it says you are not judging.',
          },
        },
      },
    },
    best: { type: 'string' },
    blocking: {
      type: 'array',
      description: 'Objections that would make this lens refuse to build the named proposal, its own pick included. Empty if none. This is a refusal, not a price — a preference goes in changeMyMindBy.',
      items: {
        type: 'object', additionalProperties: false, required: ['against', 'objection'],
        properties: {
          against: { type: 'string', description: 'The proposal id this objection is against.' },
          objection: { type: 'string' },
        },
      },
    },
    moved: { type: 'string', enum: ['first-round', 'changed', 'held'], description: 'Whether the other lenses moved your vote this round.' },
    counterArguments: {
      type: 'array',
      description: 'Round 1: empty. Later: one entry per point from another lens that you answer — for or against. Argue with them; that is what this round is for.',
      items: {
        type: 'object', additionalProperties: false, required: ['lens', 'theirPoint', 'stance', 'argument'],
        properties: {
          lens: { type: 'string', description: 'Whose point you are answering.' },
          theirPoint: { type: 'string', description: 'Their claim, stated fairly enough that they would recognise it.' },
          stance: { type: 'string', enum: ['agree', 'disagree', 'agree-but-outweighed'] },
          argument: { type: 'string', description: 'Why. Evidence from the proposals or the code, not assertion.' },
        },
      },
    },
  },
}

const MAP_FIX = {
  type: 'object', additionalProperties: false,
  required: ['fixed', 'changes', 'ticketEdits', 'needsTicketRuling'],
  properties: {
    fixed: { type: 'boolean', description: 'True only if the proposal file was edited so the flagged clauses are now owned cleanly.' },
    changes: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'change'],
        properties: {
          clause: { type: 'integer' },
          change: { type: 'string', description: 'What moved, and between which children.' },
        },
      },
    },
    ticketEdits: {
      type: 'array',
      description: 'Split directives rewritten in the parent ticket file. Empty when the parent was not touched.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'before', 'after', 'why', 'newClauseText'],
        properties: {
          clause: { type: 'integer' },
          before: { type: 'string', description: 'The directive as it read, quoted.' },
          after: { type: 'string', description: 'What it reads now.' },
          why: { type: 'string', description: 'Why the split cannot satisfy the old wording.' },
          newClauseText: { type: 'string', description: 'The whole clause after the edit, verbatim, as later stages must quote it.' },
        },
      },
    },
    needsTicketRuling: {
      type: 'array',
      description: 'Empty unless a REQUIREMENT in the parent — what must change, where, or what goes red — is what blocks the split. Those you may not touch. Filling this means the proposal and the ticket were both left unedited.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'contradiction', 'whyNoSplitSatisfiesIt'],
        properties: {
          clause: { type: 'integer' },
          contradiction: { type: 'string', description: "Quote the part of the clause that forbids the arrangement." },
          whyNoSplitSatisfiesIt: { type: 'string' },
        },
      },
    },
  },
}

const CLAUSE_MAP = {
  type: 'object', additionalProperties: false,
  required: ['rows', 'unowned', 'overlaps'],
  properties: {
    rows: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'owners', 'covered'],
        properties: {
          clause: { type: 'integer' },
          owners: {
            type: 'array',
            items: {
              type: 'object', additionalProperties: false, required: ['childKey', 'portion'],
              properties: { childKey: { type: 'string' }, portion: { type: 'string' } },
            },
          },
          covered: { type: 'boolean', description: 'True when the owners together account for the whole clause with nothing left over.' },
        },
      },
    },
    unowned: { type: 'array', items: { type: 'integer' }, description: 'Clauses no child owns, or owns only part of with the rest unclaimed.' },
    overlaps: {
      type: 'array', description: 'Where two children would do the same work. A clause split into distinct portions is NOT an overlap.',
      items: {
        type: 'object', additionalProperties: false, required: ['clause', 'childKeys', 'why'],
        properties: { clause: { type: 'integer' }, childKeys: { type: 'array', items: { type: 'string' } }, why: { type: 'string' } },
      },
    },
  },
}

const DRAFT = {
  type: 'object', additionalProperties: false,
  required: ['childKey', 'path', 'title', 'clauseCount'],
  properties: {
    childKey: { type: 'string' },
    path: { type: 'string', description: 'Absolute path of the ticket text written.' },
    title: { type: 'string' },
    clauseCount: { type: 'integer', description: 'How many Done-when clauses the drafted ticket has.' },
  },
}

const AUDIT = {
  type: 'object', additionalProperties: false,
  required: ['childKey', 'verdict', 'findings'],
  properties: {
    childKey: { type: 'string' },
    verdict: { type: 'string', enum: ['CLEAN', 'FINDINGS'] },
    findings: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['id', 'where', 'problem', 'fix'],
        properties: {
          id: { type: 'string', description: 'Stable slug for this finding so a repeat is recognisable across rounds. Same defect, same id.' },
          where: { type: 'string', description: 'Which part of the ticket: a clause number, "description", or a section name.' },
          problem: { type: 'string' },
          fix: { type: 'string', description: 'What the corrected text must say. Correct facts; never drop a requirement.' },
        },
      },
    },
  },
}

const FIX_RESULT = {
  type: 'object', additionalProperties: false,
  required: ['childKey', 'resolved', 'notResolved', 'clauseCountBefore', 'clauseCountAfter', 'requirementsRemoved'],
  properties: {
    childKey: { type: 'string' },
    resolved: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['findingId', 'whatChanged'],
        properties: { findingId: { type: 'string' }, whatChanged: { type: 'string', description: 'What the text says now that it did not before.' } },
      },
    },
    notResolved: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['findingId', 'why'],
        properties: { findingId: { type: 'string' }, why: { type: 'string', description: 'Why the finding was left standing. "The finding is wrong, and here is the evidence" is a legitimate answer.' } },
      },
    },
    clauseCountBefore: { type: 'integer' },
    clauseCountAfter: { type: 'integer' },
    requirementsRemoved: { type: 'array', items: { type: 'string' }, description: 'Any requirement this fix deleted rather than corrected. Must be empty. A finding is never resolved by deleting what it is about.' },
  },
}

const FINISH = {
  type: 'object', additionalProperties: false,
  required: ['relationsAdded', 'labelRemoved', 'commentPosted'],
  properties: {
    relationsAdded: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['blocked', 'blockedBy'],
        properties: { blocked: { type: 'string' }, blockedBy: { type: 'string' } },
      },
    },
    labelRemoved: { type: 'boolean', description: 'True when Needs Splitting is off the parent.' },
    commentPosted: { type: 'boolean' },
  },
}

const FILED = {
  type: 'object', additionalProperties: false,
  required: ['childKey', 'identifier', 'url', 'title'],
  properties: {
    childKey: { type: 'string' },
    identifier: { type: 'string', description: 'e.g. GTW-1180' },
    url: { type: 'string' },
    title: { type: 'string' },
  },
}

// --- Read ------------------------------------------------------------------

phase('Read')
log(`Scratch: ${SCRATCH}`)

const parent = await agent(
  `Read ${TICKET} on the GDTF board — the WHOLE ticket, not just its clauses — and read every docs file it names as canon.

1. Fetch it with \`includeRelations: true\`, and fetch its complete comment thread.
2. Return the description verbatim. Every section: the framing, the state-of-the-tree notes, the out-of-scope list. A summary here makes every later stage work from the wrong text.
3. Return the Done-when clauses verbatim, numbered as the ticket numbers them.
4. Return every comment verbatim. A comment opening with a \`**[name]**\` source line was written by that agent; one with no source line was typed by the owner and is the only kind that can carry a ruling — see \`${REPO}/.claude/rules/linear-discipline.md\`.
5. Return the constraints: anything in the description or the comments that fixes ordering, or says what must land before what.
6. Return what the ticket explicitly puts out of scope.

Then write the whole ticket — description and numbered clauses, verbatim — to \`${SCRATCH}/ticket.md\`, so a later stage can amend a split directive in it without going back to the board. Write nothing else. Change nothing on the board.`,
  { label: `read:${TICKET}`, phase: 'Read', schema: PARENT },
)
if (!parent) throw new Error('could not read the parent ticket')
log(`${TICKET}: ${parent.clauses.length} clauses, ${parent.comments.length} comments, ${parent.canonPaths.length} canon docs`)

const clauseBrief = parent.clauses.map((c) => `${c.n}. ${c.text}`).join('\n')
const ownerRulings = parent.comments.filter((c) => c.source === 'owner').map((c) => c.body).join('\n\n')
const CONTEXT = `Parent: ${TICKET} — "${parent.title}"

Its Done-when clauses, verbatim:
${clauseBrief}

Canon docs: ${parent.canonPaths.join(', ') || 'none named'}
Stated constraints: ${parent.constraints.join(' | ') || 'none'}
Explicitly out of scope: ${parent.outOfScope.join(' | ') || 'nothing stated'}
${ownerRulings ? `\nOwner rulings on the ticket, which bind:\n${ownerRulings}\n` : ''}`

// --- Propose / Edit / Revise ----------------------------------------------

let proposals = []

if (START_FROM === 'propose') {
  phase('Propose')
  const ANGLES = [
    'Split by what becomes usable: each child ends with something a person or an agent can do that they could not before.',
    'Split by layer of the system, deepest first, so each child is one complete layer with its own tests.',
    'Split for migration safety: order so the tree is never half-migrated, even if the first child is dull and only adds the new shape beside the old one.',
    'Split so the riskiest, least understood part lands first and alone, proven before anything is built on it.',
    'Split for the smallest first child that is still worth landing on its own, then whatever follows.',
  ]

  proposals = (await parallel(
    Array.from({ length: PROPOSAL_COUNT }, (_, i) => () =>
      agent(
        `Propose ONE way to split ${TICKET} into children that each land green on their own.

${CONTEXT}

The principle YOUR proposal is organised around — yours alone, and not the only valid one:
${ANGLES[i % ANGLES.length]}

Read the canon docs and the code you need to answer honestly. The test that decides a split:
**with child K landed and nothing after it built, is the tree green and is nothing half-done?**
Say why for every child. That is the \`greenBecause\` field, and a vague one is the tell for a split that does not work.

Every parent clause must be accounted for. A clause may be split across children when it genuinely contains two pieces of work — say which portion each child owns, in the clause's own words. Do not split a clause to make the arithmetic come out.

${HOUSE}

Write the proposal in full to \`${SCRATCH}/proposal-${i + 1}.md\` — the reasoning, the children, the order, what you rejected and why — then return the structured form. Do not touch the board.`,
        { label: `propose-${i + 1}`, phase: 'Propose', schema: PROPOSAL },
      )),
  )).filter(Boolean)
  if (!proposals.length) throw new Error('no proposal survived')

  phase('Edit')
  const notes = (await parallel(proposals.map((p) => () =>
    agent(
      `You are the editor for ONE proposal to split ${TICKET}. Make it the best version of itself.

Proposal ${p.id}, at \`${p.path}\`.
Its organising principle, which you do not change: ${p.angle}

${CONTEXT}

You are not ranking it against anything and you are not choosing whether it wins. Another stage does that. Your job is to find what would make this proposal stronger on its own terms.

Read the file and the code. Say what it gets right that a rewrite must not lose, then give specific changes — each naming the child it applies to, or \`split\` when the shape itself is wrong. Every change carries its reason.

The thing most worth your attention: a child whose \`greenBecause\` does not survive contact with the code. Check them.

${HOUSE}`,
      { label: `editor-${p.id}`, phase: 'Edit', schema: EDIT_NOTES },
    ))
  )).filter(Boolean)

  phase('Revise')
  proposals = (await parallel(proposals.map((p) => () => {
    const note = notes.find((n) => n.id === p.id)
    return agent(
      `Revise the split proposal at \`${p.path}\`.

Its organising principle, which you keep: ${p.angle}

${CONTEXT}

Keep, per its editor: ${note?.keep ?? 'whatever in it is sound'}

Apply these changes:
${(note?.fix ?? []).map((f, i) => `${i + 1}. [${f.child}] ${f.change}\n   why: ${f.why}`).join('\n') || 'none given — tighten every greenBecause until it names the specific thing that would otherwise be half-done'}

Rewrite the file in place at \`${p.path}\` and return the structured form. Every parent clause accounted for, whole or in named portions.

${HOUSE}`,
      { label: `revise-${p.id}`, phase: 'Revise', schema: PROPOSAL },
    )
  }))).filter(Boolean)
} else if (START_FROM === 'vote') {
  log(`startFrom=vote: reading ${A.proposalPaths.length} supplied proposals`)
  proposals = (await parallel(A.proposalPaths.map((path, i) => () =>
    agent(
      `Read the split proposal at \`${path}\` and return it in structured form. Do not revise it, do not judge it, do not write anything.

${CONTEXT}

Use \`p${i + 1}\` as the id if the file does not name one, and \`${path}\` as the path.`,
      { label: `load-proposal-${i + 1}`, phase: 'Vote', schema: PROPOSAL },
    ))
  )).filter(Boolean)
}

// --- Vote / Argue ----------------------------------------------------------

let history = []
let votes = []
let chosen = null

let winnerPath = null

if (START_FROM === 'map') {
  log(`startFrom=map: building from ${A.winner}`)
  winnerPath = A.winner
} else {
  phase('Vote')
  const proposalList = proposals.map((p) => `- ${p.id}: \`${p.path}\` — ${p.angle}`).join('\n')
  let round = 0

  while (round < MAX_VOTE_ROUNDS) {
    round += 1
    const prior = round === 1 ? '' : `
The other lenses voted last round. Read what they argued and answer it:
${votes.map((v) => `\n### ${v.lens} — picked ${v.best}\n${v.ranking.map((r) => `  ${r.id}: ${r.score}/10 — ${r.why}\n    would change its mind by: ${r.changeMyMindBy}`).join('\n')}\n  blocking: ${v.blocking.map((b) => `[${b.against}] ${b.objection}`).join('; ') || 'none'}`).join('\n')}

Every lens named its price last round. Read the prices on the proposal you picked: if another lens named a condition and this round's proposals meet it, say so. If one named a condition against your pick, answer it or meet it.

You may hold your position or change it. Both are legitimate. Say which, and why. Do not converge for the sake of converging — a disagreement that survives scrutiny is worth more than an agreement that was reached to be agreeable. But a price you named and then ignored when it was paid is not a position, so honour your own.
`

    votes = (await parallel(LENSES.map((lens) => () =>
      agent(
        `Judge the proposals for splitting ${TICKET} through ONE lens and no other.

Your lens — **${lens.key}**: ${lens.question}

Proposals:
${proposalList}

${CONTEXT}
${prior}
Read the proposal files, the canon and the code. Score every proposal 0-10 against YOUR lens alone — three other lenses are covering the concerns that are not yours, and a score that quietly folds their concerns in makes the panel useless. Name the proposal you would build.

For every proposal, name your price in \`changeMyMindBy\`: what would make you drop the one you picked, and what would make you pick each one you did not. A price is a change to the proposal someone could actually make, not a wish — and once it is paid you are expected to move.

Separately, list in \`blocking\` any objection that would make you refuse to build a proposal at all, your own pick included, and say which proposal each is against. A refusal is not a preference; if you would still build it, it belongs in your price.

${HOUSE}`,
        { label: `vote-${lens.key}-r${round}`, phase: round === 1 ? 'Vote' : 'Argue', schema: VOTE },
      ))
    )).filter(Boolean)

    const tally = {}
    for (const v of votes) tally[v.best] = (tally[v.best] ?? 0) + 1
    history.push({
      round,
      picks: votes.map((v) => ({ lens: v.lens, best: v.best, blocking: v.blocking.length })),
      tally,
      scores: votes.map((v) => ({ lens: v.lens, ranking: v.ranking })),
    })
    log(`round ${round}: ${votes.map((v) => `${v.lens}→${v.best}${v.blocking.length ? '!' : ''}`).join(', ')}`)

    // Consensus is about the winner only. A lens refusing to build a proposal nobody picked
    // is not a disagreement about what to build, so it must not hold the panel open.
    const unanimous = Object.entries(tally).find(([, n]) => n === votes.length)
    const refusedByAny = unanimous
      && votes.some((v) => v.blocking.some((b) => b.against === unanimous[0]))
    if (unanimous && !refusedByAny) {
      chosen = proposals.find((p) => p.id === unanimous[0])
      log(`consensus on ${unanimous[0]} after ${round} round(s)`)
      break
    }
    if (round === 1) phase('Argue')
  }

  // The panel's findings outlive this run: a hand-picked winner is re-entered as a fresh
  // process, and the objections that cost it consensus are exactly what it needs revising
  // against. The script has no filesystem, so an agent writes them.
  await agent(
    `Write this vote record to \`${SCRATCH}/votes-final.json\` as JSON, verbatim, and return the path. Do not summarise it, do not judge it, do not read anything else.

\`\`\`json
${JSON.stringify({ ticket: TICKET, winner: chosen?.id ?? null, rounds: history.length, votes }, null, 2)}
\`\`\``,
    { label: 'record-votes', phase: chosen ? 'Settle' : 'Argue' },
  )

  if (!chosen) {
    log(`no consensus after ${MAX_VOTE_ROUNDS} rounds — returning the disagreement`)
    return {
      ticket: TICKET,
      outcome: 'no-consensus',
      rounds: history,
      finalVotes: votes,
      proposals: proposals.map((p) => ({ id: p.id, path: p.path, angle: p.angle })),
      scratch: SCRATCH,
      resume: `Workflow({scriptPath: '.claude/workflows/split-ticket.js', args: {ticket: '${TICKET}', startFrom: 'map', winner: '<the proposal file you choose>', scratch: '${SCRATCH}'}})`,
    }
  }
  winnerPath = chosen.path
}

// --- Settle ----------------------------------------------------------------
// A winner still carries every objection the losing lenses raised against it. Fold them in
// before anyone writes a ticket from it, on both paths — the hand-picked winner needs this
// most, because it lost the vote it was picked out of.

phase('Settle')
await agent(
  `Revise the chosen split proposal at \`${winnerPath}\` in place, so the ticket writers work from a document that answers the panel. Keep its shape and its voice.

${CONTEXT}

The panel's final vote is at \`${SCRATCH}/votes-final.json\` — read it. If it is not there, say so and revise against the proposal and the code alone.

Apply, in this order:

1. **Every \`blocking\` objection whose \`against\` names this proposal.** A lens raising one said it would refuse to build this split. Fix the proposal so the objection no longer holds, or state in the proposal why it does not apply.
2. **Every \`changeMyMindBy\` price named against this proposal** by a lens that did not pick it. That is the change which would have won its vote, so it is the cheapest real improvement available.
3. **Every point in \`counterArguments\` where each lens that spoke to it answered \`agree\` or \`agree-but-outweighed\`.** The panel converged there; apply it.

Leave alone any point where the stances conflict. That is the disagreement the vote or the owner already settled, and re-opening it undoes the decision.

Change nothing else. Do not re-argue the split, do not compare it to the proposals that lost, do not touch the board. Open the code and check every symbol and path you write still exists.

${HOUSE}`,
  { label: 'settle-winner', phase: 'Settle' },
)

// --- Map -------------------------------------------------------------------

phase('Map')
chosen = await agent(
  `Read the split proposal at \`${winnerPath}\` and return it in structured form. It has already been chosen and revised — do not judge it, do not revise it further, write nothing.

${CONTEXT}

Every parent clause must appear in some child's \`ownsClauses\`, whole or as a named portion. If the file leaves a clause unaccounted for, return it as unowned by saying so in \`risks\` rather than inventing an owner.`,
  { label: 'load-winner', phase: 'Map', schema: PROPOSAL },
)
if (!chosen) throw new Error(`could not read the winning proposal at ${winnerPath}`)

const mapPrompt = (children) => `Build the clause-ownership map for the winning split of ${TICKET}, at \`${winnerPath}\`.

${CONTEXT}

Children: ${children.map((c) => `\`${c.key}\` — ${c.title}`).join('; ')}

One row per parent clause. For each, name every child that owns any part of it and what portion. A clause split into distinct portions across children is legitimate — mark \`covered\` true when the portions together account for the whole clause with nothing left over.

Report as \`unowned\` any clause no child owns, and any clause only partly claimed with the rest left unclaimed. Report as \`overlaps\` only where two children would genuinely do the same work — a clean partition is not an overlap.

Report. Do not fix, do not rewrite the proposal, do not touch the board.`

let map = null
let mapRounds = 0
let ticketRuling = []
const ticketEdits = []

while (mapRounds < MAX_MAP_ROUNDS) {
  mapRounds += 1
  map = await agent(mapPrompt(chosen.children), { label: `clause-map-r${mapRounds}`, phase: 'Map', schema: CLAUSE_MAP })

  if (map && !map.unowned.length && !map.overlaps.length) {
    if (mapRounds > 1) log(`clause map clean after ${mapRounds} rounds`)
    break
  }
  log(`map round ${mapRounds}: unowned [${(map?.unowned ?? []).join(', ')}], overlaps [${(map?.overlaps ?? []).map((o) => o.clause).join(', ')}]`)
  if (mapRounds === MAX_MAP_ROUNDS) break

  const fix = await agent(
    `The clause-ownership map of the split at \`${winnerPath}\` is not clean. Fix the proposal so it is.

${CONTEXT}

Unowned clauses — no child owns them, or a child claims part and the rest is unclaimed:
${(map?.unowned ?? []).map((n) => `- clause ${n}`).join('\n') || '- none'}

Overlaps — two children doing the same work:
${(map?.overlaps ?? []).map((o) => `- clause ${o.clause}: ${o.childKeys.join(' and ')} — ${o.why}`).join('\n') || '- none'}

Edit the proposal in place: move work between children, split a child, or state a portion the proposal left implied. Keep its shape and its voice, change nothing the map did not flag, and check every symbol and path you write against the tree.

Sometimes the split is fine and the parent is what blocks it. A clause can hold two different things, and they are not equal:

- **A requirement** — what must change, where, and what goes red if it is wrong. This is the contract. **You may never edit it, and never narrow it to make the split fit** (\`${REPO}/.claude/rules/design-fidelity.md\`). If a requirement is what no arrangement of children can satisfy, leave everything alone, return it in \`needsTicketRuling\`, and say why. Stopping is the right answer there.
- **A split directive** — the ticket telling the splitter how to split: which child owns what, what lands with what. That is this workflow's business, not the ticket's, and it is often written before anyone knew what the children would be. You may rewrite one.

To rewrite a directive, edit \`${SCRATCH}/ticket.md\` in place and record it in \`ticketEdits\`: the clause, the old wording quoted, the new wording, why the split cannot satisfy the old one, and the whole amended clause verbatim — later stages quote clause text word for word, so \`newClauseText\` must be exactly what they should quote. Change no requirement in that file, and touch no clause the map did not flag.

Worked example, so the line is clear. A clause reading "the child that lands this owns all of them" for a list of fixtures is a directive: it says who does the work, not what the work is. Rewriting it to carve out one fixture is legal. Deleting a fixture from the list, or dropping the behaviour the clause requires, is not.

${HOUSE}`,
    { label: `fix-map-r${mapRounds}`, phase: 'Map', schema: MAP_FIX },
  )

  if (fix?.needsTicketRuling?.length) {
    ticketRuling = fix.needsTicketRuling
    log(`map round ${mapRounds}: the contradiction is in ${TICKET}, not the split — stopping for a ruling`)
    break
  }
  if (!fix?.fixed) {
    log(`map round ${mapRounds}: fix agent changed nothing — stopping`)
    break
  }

  // A rewritten directive changes the clause text the Write stage quotes verbatim, so the
  // in-memory copy has to follow the file.
  for (const e of fix.ticketEdits ?? []) {
    const clause = parent.clauses.find((c) => c.n === e.clause)
    if (clause) clause.text = e.newClauseText
    ticketEdits.push(e)
  }
  log(`map round ${mapRounds}: ${fix.changes.length} split change(s), ${(fix.ticketEdits ?? []).length} ticket directive(s) — remapping`)

  // The fix rewrote the file, so the structured read of it is stale.
  chosen = await agent(
    `Read the split proposal at \`${winnerPath}\` and return it in structured form. It has just been revised — do not judge it, do not revise it further, write nothing.

${CONTEXT}

Every parent clause must appear in some child's \`ownsClauses\`, whole or as a named portion. If the file leaves a clause unaccounted for, return it as unowned by saying so in \`risks\` rather than inventing an owner.`,
    { label: `reload-winner-r${mapRounds}`, phase: 'Map', schema: PROPOSAL },
  )
  if (!chosen) throw new Error(`could not re-read the proposal at ${winnerPath} after fix round ${mapRounds}`)
}

if (!map || map.unowned.length || map.overlaps.length) {
  log(`clause map not clean after ${mapRounds} round(s)`)
  return {
    ticket: TICKET,
    outcome: ticketRuling.length ? 'needs-ticket-ruling' : 'clause-map-unclean',
    winner: chosen.id,
    winnerPath,
    map,
    mapRounds,
    ticketRuling,
    voteRounds: history,
    scratch: SCRATCH,
    resume: `${ticketRuling.length ? `Rule on ${TICKET} and amend it` : `Fix the proposal at ${winnerPath}`}, then: Workflow({scriptPath: '.claude/workflows/split-ticket.js', args: {ticket: '${TICKET}', startFrom: 'map', winner: '${winnerPath}', scratch: '${SCRATCH}'}})`,
  }
}

// --- Write / Audit / Fix ---------------------------------------------------

phase('Write')
const drafts = (await pipeline(
  chosen.children,
  (child) => {
    const owned = (child.ownsClauses ?? []).map((o) => {
      const c = parent.clauses.find((pc) => pc.n === o.clause)
      return `Parent clause ${o.clause} — owns: ${o.portion}\n"""\n${c ? c.text : '(clause text missing)'}\n"""`
    }).join('\n\n')
    return agent(
      `Write the complete ticket for ONE child of ${TICKET}: description, context and Done-when clauses.

Child \`${child.key}\` — ${child.title}
Delivers: ${child.delivers}
Green on its own because: ${child.greenBecause}
Lands after: ${child.dependsOn.join(', ') || 'nothing'}

The parent clauses this child owns, verbatim. Write to THESE WORDS, not to the summary above — a ticket written from a summary is how scope narrows:

${owned || '(this child owns no clause — say so and stop)'}

${CONTEXT}
Canon: ${parent.canonPaths.join(', ')}. The full split is at \`${chosen.path}\`.

Open the code and check every symbol and path you cite exists right now. Say what changes, where, and what goes red if it is wrong — \`${REPO}/.claude/rules/clause-writing.md\` is the list, and the audit checks against the same one.

If this child adds a player or author verb, include the MCP surface block from \`${REPO}/.claude/rules/linear-discipline.md\`. A Feature / Editor / Improvement draft without that block is incomplete — **NEVER** omit it.

Do not restate the parent. Do not claim scope this child does not own. Do not name a mutation that would go red for a reason other than the clause it belongs to.

${HOUSE}

Write the ticket text to \`${SCRATCH}/child-${child.key}.md\` and return the structured form. Do not touch the board.`,
      { label: `write-${child.key}`, phase: 'Write', schema: DRAFT },
    )
  },
  async (draft, child) => {
    if (!draft) return null
    const seen = new Set()
    let rounds = 0
    while (rounds < MAX_AUDIT_ROUNDS) {
      rounds += 1
      const audit = await agent(
        `Audit the drafted ticket at \`${draft.path}\`, a child of ${TICKET}. Audit the WHOLE ticket — its description, its context sections and its Done-when clauses.

${CONTEXT}
Canon: ${parent.canonPaths.join(', ')}.

Open every file it cites and confirm the symbol is there now. Check each clause is buildable as written, names where the change goes, and names what goes red. Check nothing it claims contradicts the canon or the parent. If the child adds a player or author verb, the MCP surface block must be present and name host, the verb, grow-or-add-or-none, and a drive path.

Report findings only — you do not rewrite. Give each finding a stable id slug describing the defect, so the same defect reported twice carries the same id.`,
        { label: `audit-${child.key}-r${rounds}`, phase: 'Audit', schema: AUDIT, agentType: 'clause-audit' },
      )
      const findings = audit?.findings ?? []
      const fresh = findings.filter((f) => !seen.has(f.id))
      if (!fresh.length) return { ...draft, auditRounds: rounds, clean: true }
      fresh.forEach((f) => seen.add(f.id))
      const fix = await agent(
        `Fix the drafted ticket at \`${draft.path}\` for child \`${child.key}\`.

Findings to resolve, each by its id:
${fresh.map((f) => `- ${f.id} [${f.where}] ${f.problem}\n  must say: ${f.fix}`).join('\n')}

Count the Done-when clauses before you start and after you finish, and report both.

Rewrite the file in place. Correct the facts. **Never resolve a finding by deleting what it is about** — a clause that looks wrong stays, and you say so in the ticket. \`${REPO}/.claude/rules/design-fidelity.md\`: narrowing a requirement to make it fit is the failure this kit exists to stop, and a fix round is where it happens.

Report every finding as resolved or not resolved. Leaving one standing because it is wrong is legitimate — give the evidence.

${HOUSE}`,
        { label: `fix-${child.key}-r${rounds}`, phase: 'Audit', schema: FIX_RESULT },
      )
      if (fix && (fix.requirementsRemoved.length || fix.clauseCountAfter < fix.clauseCountBefore)) {
        log(`fix-${child.key}-r${rounds} shrank the ticket: ${fix.clauseCountBefore}→${fix.clauseCountAfter} clauses, removed [${fix.requirementsRemoved.join('; ')}]`)
        return { ...draft, auditRounds: rounds, clean: false, shrank: true, fix }
      }
    }
    return { ...draft, auditRounds: rounds, clean: false }
  },
)).filter(Boolean)

const dirty = drafts.filter((d) => !d.clean)
if (dirty.length) log(`hit the audit cap, not clean: ${dirty.map((d) => d.childKey).join(', ')}`)

// --- File ------------------------------------------------------------------

phase('File')
const filed = (await parallel(drafts.map((d) => () => {
  const child = chosen.children.find((c) => c.key === d.childKey)
  return agent(
    `File ONE ticket on the GDTF board from the text at \`${d.path}\`.

- Description: that file's contents, verbatim. Do not summarise it and do not add a source line — a description is understood to be agent-written (\`${REPO}/.claude/rules/linear-discipline.md\`).
- Title: ${d.title}
- Parent: ${TICKET}
- Labels: carry the parent's labels ${JSON.stringify(parent.labels)} EXCEPT \`Epic\` and \`Needs Splitting\`. Pass \`team: GDTF\` when you list labels — without it the team-scoped ones are silently missing.
- Status: Backlog.

Add no relations — a later step adds them once every sibling exists.

Return the identifier, url and title.`,
    { label: `file-${d.childKey}`, phase: 'File', schema: FILED },
  )
}))).filter(Boolean)

const keyToId = {}
for (const f of filed) keyToId[f.childKey] = f.identifier

const edges = chosen.children
  .filter((c) => (c.dependsOn ?? []).length && keyToId[c.key])
  .map((c) => `${keyToId[c.key]} is blocked by ${c.dependsOn.map((k) => keyToId[k] ?? `(${k} not filed)`).join(' and ')}`)

const finish = await agent(
  `Finish the split of ${TICKET} on the board.

1. Add these blocking relations, reciprocal on both tickets:
${edges.join('\n') || '(none — the children are independent)'}
2. Remove the \`Needs Splitting\` label from ${TICKET}. Leave \`Epic\` on it and leave its status alone.
${ticketEdits.length ? `3. Amend ${TICKET}'s description with the split directives the map forced. For each, replace the old wording with the new, and change nothing else in the clause:
${ticketEdits.map((e) => `\n- Clause ${e.clause}. Was: "${e.before}"\n  Now: "${e.after}"\n  Because: ${e.why}`).join('')}

Read the live description first and apply these to it — the ticket may have moved since this run read it. If the old wording is not there, do not force it: say so in your report and leave that clause alone.
4.` : '3.'} Comment on ${TICKET}, opening with the source line \`**[split-ticket]**\` and nothing above it:

Split into ${filed.length} children, in build order:
${filed.map((f) => `- ${f.identifier} — ${f.title}`).join('\n')}

Each child was written from the parent clauses it owns and audited until the audit returned nothing new.${dirty.length ? ` These hit the audit cap and were filed with findings outstanding: ${dirty.map((d) => keyToId[d.childKey] ?? d.childKey).join(', ')}.` : ''}${ticketEdits.length ? `

Say in the comment which split directives were rewritten and why, listing the clause numbers: ${ticketEdits.map((e) => e.clause).join(', ')}. No requirement was changed — only who owns what.` : ''}

Report each relation you added, whether the label came off,${ticketEdits.length ? ' which directive edits applied,' : ''} and whether the comment posted.`,
  { label: 'finish-parent', phase: 'File', schema: FINISH },
)

return {
  ticket: TICKET,
  outcome: 'filed',
  winner: chosen.id,
  winnerPath: chosen.path,
  voteRounds: history,
  clauseMap: map,
  mapRounds,
  ticketEdits,
  children: filed.map((f) => {
    const d = drafts.find((x) => x.childKey === f.childKey)
    return { ...f, clean: d?.clean, auditRounds: d?.auditRounds, draftPath: d?.path, shrank: d?.shrank ?? false }
  }),
  notClean: dirty.map((d) => d.childKey),
  shrank: drafts.filter((d) => d.shrank).map((d) => ({ childKey: d.childKey, fix: d.fix })),
  finish,
  scratch: SCRATCH,
}

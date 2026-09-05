export const meta = {
  name: 'research-and-design',
  description: 'Research one open design question to a judged proposal and a judged software design, then post both to Linear',
  phases: [
    { title: 'Options', detail: 'find the field, no candidates supplied' },
    { title: 'Validate options', detail: 'check each option is real, loop until clean' },
    { title: 'Propose', detail: 'one proposal per surviving option' },
    { title: 'Fit check', detail: 'review and revise each proposal, loop until sound' },
    { title: 'Judge proposals', detail: 'five lenses, argue to convergence' },
    { title: 'Design', detail: 'software design for the top three' },
    { title: 'Review design', detail: 'review and revise each design, loop until clean' },
    { title: 'Judge designs', detail: 'five lenses, argue to convergence' },
    { title: 'Close conditions', detail: 'satisfy any conditional agreement' },
    { title: 'Publish', detail: 'post proposal and design to the ticket' },
  ],
}

const A = args
const MAX_ROUNDS = 20
// Stop after the proposal is chosen. No software design, no design judging.
const PROPOSAL_ONLY = A.proposalOnly === true
// Start from an options file already on disk, skipping stages 1 and 2.
const USE_EXISTING_OPTIONS = A.useExistingOptions === true
// The proposal is already decided. Skip stages 1 to 6 and design it.
const DESIGN_ONLY = A.designOnly === true
const ROOT = `${A.scratch}/research/${A.subject}`

log(`${A.ticket} — ${A.title}`)
log(`Question: ${A.question}`)
log(`Working folder: ${ROOT}`)
if (PROPOSAL_ONLY) log('Proposal only: stages 7 to 11 are skipped')
if (USE_EXISTING_OPTIONS) log(`Using the options already in ${ROOT}/options.json: stages 1 and 2 are skipped`)
if (DESIGN_ONLY) log(`Design only: the proposal is settled, stages 1 to 6 are skipped`)
if (DESIGN_ONLY && PROPOSAL_ONLY) {
  return { ticket: A.ticket, outcome: 'FAILED', reason: 'proposalOnly and designOnly are mutually exclusive' }
}
if (DESIGN_ONLY && !A.proposalKey) {
  return { ticket: A.ticket, outcome: 'FAILED', reason: 'designOnly needs proposalKey naming the settled proposal' }
}

// Hoisted so the research half can be skipped wholesale.
let options = null
let optionList = []
let optionsRound = 0
let optionsDeadlocked = false
let survivors = []
let abandoned = []
let proposalJudging = null
let topThree = []

if (!DESIGN_ONLY) {

const PLAIN = `.claude/rules/plain-language.md binds every word you write into a file: no em dashes, no bold
lead-in bullets, no "it is not X it is Y", no consultant nouns, no editorialising, no closers that sound like a
conclusion and say nothing. Short sentences, concrete nouns, competent reader.`

const REPO = `The project is gdtf, a turn-based tactics situation generator in Rust and Bevy, Necromunda crossed
with XCOM. The sim crate gdtf_battle_sim is the source of combat truth and is render-free and deterministic.
Anything a player can do the AI can also do, through the same Acts. Anything a player or author can do, the
matching MCP host must also be able to drive.`

// ---------------------------------------------------------------- stage 1

phase('Options')

const OPTIONS_SCHEMA = {
  type: 'object',
  properties: {
    options: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          key: { type: 'string' },
          name: { type: 'string' },
          summary: { type: 'string' },
          family: { type: 'string' },
          shippedIn: { type: 'array', items: { type: 'string' } },
          sources: { type: 'array', items: { type: 'string' } },
        },
        required: ['key', 'name', 'summary', 'family', 'sources'],
      },
    },
    fieldNotes: { type: 'string' },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['options', 'fieldNotes', 'filesWritten'],
}

options = USE_EXISTING_OPTIONS
  ? await agent(
      `Load an options list that already exists and has been curated by hand. Do not research, do not add, do
not remove, and do not reword anything.

Read ${ROOT}/options.json and return its contents exactly, matching the schema. Every option in the file comes
back in your return value, with its key, name, summary, family, shippedIn and sources unchanged.

If the file is missing or unreadable, return an empty options array and say so in fieldNotes. Do not invent a
replacement list.

Set filesWritten to an empty array. You are reading, not writing.`,
      { label: 'load-options', phase: 'Options', model: 'sonnet', schema: OPTIONS_SCHEMA },
    )
  : await agent(
  `Find the field of options for a design question. Nobody has given you a candidate list, and you must not ask
for one. Finding the options IS the task, and an option nobody thought to name is the most valuable thing you
can return.

${REPO}

The question: ${A.question}

${A.optionsBrief || ''}

Aim for ten options. Accept fewer only when fewer genuinely exist, and if you return fewer than ten say in
fieldNotes exactly why, naming what you searched and found nothing for. Do not pad with near-duplicates to
reach a count; two options that differ only in wording are one option.

A composite is a legitimate option. Many real answers are not one technique but an assignment of different
techniques to different layers of the decision: one thing choosing where to move, another choosing what a unit
does when it gets there, another coordinating a squad. If that is how a working system is actually built, list
it as one option, name the layers, and say what handles each. Do not split it into its parts and list them
separately as if they competed, and do not manufacture a composite by pairing two techniques nobody has ever
paired.

Some techniques only ever appear as one layer of such a system and cannot stand alone. A grid of weighted
squares answers where to stand and nothing else. List that as part of a composite rather than as a whole
answer.

Spread across families. If eight of your ten come from one tradition, you have surveyed one tradition rather
than the field. Include approaches from published games, from academic work, and from adjacent problem domains,
and include the option of doing the obvious simple thing, which is often absent from writing about a field
because nobody publishes it.

Each option needs a key in kebab-case, a name, one or two sentences on the state of the art, the family it
belongs to, where it has shipped if it has, and at least one real source URL. Do not write detail beyond two
sentences: a later stage develops each one.

Use the web. Load WebSearch and WebFetch through ToolSearch if they are not already available. Every source
must be a URL you actually fetched. Do not cite from memory.

Write the result to two files:
- ${ROOT}/options.json — the structured object, exactly matching what you return.
- ${ROOT}/options.md — the same content as readable prose, one short section per option.

${PLAIN}`,
  { label: 'options', phase: 'Options', model: 'fable', effort: 'high', schema: OPTIONS_SCHEMA },
)

if (!options || !options.options?.length) {
  return { ticket: A.ticket, outcome: 'FAILED', stage: 'Options', reason: 'no options returned' }
}
log(`Stage 1 returned ${options.options.length} options: ${options.options.map(o => o.key).join(', ')}`)

// ---------------------------------------------------------------- stage 2

phase('Validate options')

const VALIDATE_SCHEMA = {
  type: 'object',
  properties: {
    allValid: { type: 'boolean' },
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          key: { type: 'string' },
          verdict: { type: 'string', enum: ['REAL', 'UNVERIFIED', 'HALLUCINATED', 'TOO_LONG', 'DUPLICATE_OF', 'COMBINE_WITH'] },
          evidence: { type: 'string' },
          duplicateOf: { type: ['string', 'null'] },
          combineWith: { type: 'array', items: { type: 'string' } },
          combineRationale: { type: 'string' },
          fix: { type: 'string' },
        },
        required: ['key', 'verdict', 'evidence', 'fix'],
      },
    },
    countVerdict: { type: 'string' },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['allValid', 'verdicts', 'countVerdict', 'filesWritten'],
}

while (!USE_EXISTING_OPTIONS) {
  const check = await agent(
    `Validate a list of design options. Do not change the list. Do not add to it. Report only.

Read ${ROOT}/options.json.

For each option decide one verdict:
- REAL: the thing exists, is used, and the sources support the summary. Say which source you checked.
- UNVERIFIED: plausible but you could not confirm it from a source you fetched.
- HALLUCINATED: the thing does not exist, or the sources do not say what the summary claims. Be specific.
- TOO_LONG: the summary runs past two or three sentences, or has drifted into a proposal.
- DUPLICATE_OF: it is another option under a different name. Name that option in duplicateOf.
- COMBINE_WITH: it is real, but it only solves part of the question, and it needs a complement to stand as an
  answer. Name the options it should join in combineWith and say why in combineRationale.

On COMBINE_WITH. Some approaches answer one slice of a problem and nothing else. A grid of weighted squares
tells a unit where to stand and says nothing about what to do when it gets there. That is not a wrong option
and it is not a duplicate, it is half an answer. When you see one, look down the rest of the list for the half
that completes it and pair them. If nothing on the list completes it, still mark it COMBINE_WITH, leave
combineWith empty, and say in fix what kind of complement is missing so the next stage can go and find one.

Do not use COMBINE_WITH to merge two whole answers that happen to be compatible. Almost anything can be
combined with anything. Use it only where an option cannot stand alone as an answer to the question.

Fetch the sources. An option whose sources you did not open is UNVERIFIED, not REAL, however familiar it looks.
Load WebSearch and WebFetch through ToolSearch if needed.

In countVerdict say whether the list holds between five and ten distinct options, and if not, what is missing or
surplus.

Set allValid true only when every verdict is REAL and countVerdict is satisfied.

Write the structured result to ${ROOT}/options_feedback.json, overwriting any existing file.

${PLAIN}`,
    { label: `validate-options r${optionsRound}`, phase: 'Validate options', model: 'sonnet', schema: VALIDATE_SCHEMA },
  )

  if (!check) break
  if (check.allValid) {
    log(`Options validated clean after ${optionsRound} revision rounds`)
    break
  }
  if (optionsRound >= MAX_ROUNDS) {
    optionsDeadlocked = true
    log(`Options loop hit ${MAX_ROUNDS} rounds without validating clean`)
    break
  }
  optionsRound += 1

  const revised = await agent(
    `Revise a list of design options against a validator's feedback.

Read ${ROOT}/options.json and ${ROOT}/options_feedback.json.

Act on each verdict:
- REAL: leave the option alone.
- UNVERIFIED: find a source that confirms it, or remove it.
- HALLUCINATED: remove it.
- TOO_LONG: cut it back to two or three sentences on the state of the art.
- DUPLICATE_OF: merge it into the option it duplicates, keeping the better sources.
- COMBINE_WITH: fold the named options into one option that describes the combined approach, keeping every
  source and giving it a key and a name that say what the combination is rather than naming one half. The
  summary describes what the pieces do together, still in two or three sentences. If combineWith is empty, the
  complement has not been found yet: research one, add it, and combine. If no complement exists, remove the
  option, because half an answer cannot be proposed.

Combining reduces the count, and so does removing. If the list drops below five options, research and add new
ones to get back into the five to ten range.
A replacement is a genuinely different approach, not a restatement of one already listed. Widen the families
you search before you widen the wording.

You may disagree with a verdict. If you do, keep the option, and record the disagreement in the option's
summary field as one clause saying why the validator is wrong and naming the source that settles it. Do not
silently ignore a verdict.

Rewrite both ${ROOT}/options.json and ${ROOT}/options.md so they agree.

${PLAIN}`,
    { label: `revise-options r${optionsRound}`, phase: 'Validate options', model: 'fable', effort: 'high', schema: OPTIONS_SCHEMA },
  )
  if (revised?.options?.length) options = revised
}

optionList = options.options
log(`Proceeding with ${optionList.length} options`)

// ---------------------------------------------------------------- stages 3 and 4

phase('Propose')

const PROPOSAL_SCHEMA = {
  type: 'object',
  properties: {
    key: { type: 'string' },
    name: { type: 'string' },
    style: { type: 'string' },
    fitToBattlescape: { type: 'string' },
    pros: { type: 'array', items: { type: 'string' } },
    cons: { type: 'array', items: { type: 'string' } },
    openQuestions: { type: 'array', items: { type: 'string' } },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['key', 'name', 'style', 'fitToBattlescape', 'pros', 'cons', 'filesWritten'],
}

const FIT_SCHEMA = {
  type: 'object',
  properties: {
    key: { type: 'string' },
    verdict: { type: 'string', enum: ['GOOD', 'NEEDS_WORK', 'ABANDON'] },
    groundedPros: { type: 'array', items: { type: 'string' } },
    ungroundedPros: { type: 'array', items: { type: 'string' } },
    missedCons: { type: 'array', items: { type: 'string' } },
    contradictions: { type: 'array', items: { type: 'string' } },
    required: { type: 'array', items: { type: 'string' } },
    abandonReason: { type: 'string' },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['key', 'verdict', 'groundedPros', 'ungroundedPros', 'missedCons', 'contradictions', 'filesWritten'],
}

const REVISE_SCHEMA = {
  type: 'object',
  properties: {
    key: { type: 'string' },
    abandoned: { type: 'boolean' },
    abandonReason: { type: 'string' },
    changesMade: { type: 'array', items: { type: 'string' } },
    disagreements: {
      type: 'array',
      items: {
        type: 'object',
        properties: { point: { type: 'string' }, why: { type: 'string' } },
        required: ['point', 'why'],
      },
    },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['key', 'abandoned', 'changesMade', 'disagreements', 'filesWritten'],
}

const proposalResults = await pipeline(
  optionList,
  o => agent(
    `Develop one option into a full proposal for gdtf.

${REPO}

The question: ${A.question}

Your option, from ${ROOT}/options.json:
${JSON.stringify(o, null, 2)}

${A.proposalBrief || ''}

This is a proposal, not a software design. Argue at the level of the game: what happens on the board, what the
player and the AI can do, what the sim has to decide, what an author has to write. A later stage designs the
types and the modules, so do not write types, signatures, file paths or module layouts here, and do not cite
symbols or line numbers. Naming a subsystem in plain terms is right; naming a struct is not.

Read enough of the repo and docs/ to avoid saying something false about how gdtf works today. Use that reading
to keep yourself honest, not to fill the proposal with implementation.

Read the other options in options.json so you know what you are distinct from, but argue only for yours.

Write four things:
- What this style is, in general terms, for a reader who has not met it. If it is a composite, say which
  technique owns which layer of the decision and where one hands over to the next, because the handover is
  usually where a composite goes wrong.
- How it fits gdtf's battlescape: what it changes about how a turn plays, what the AI and the player each get
  from it, what an author would have to write, and what it leaves alone.
- Its pros, each one grounded in something real rather than asserted.
- Its cons, honestly. A proposal with no real cons has not been thought about, and the review stage will say so.

Also list the questions this proposal cannot answer by itself.

Write the prose to ${ROOT}/proposals/${o.key}/proposal.md and the structured form to
${ROOT}/proposals/${o.key}/proposal.json.

${PLAIN}`,
    { label: `propose:${o.key}`, phase: 'Propose', model: 'opus', effort: 'high', schema: PROPOSAL_SCHEMA },
  ),
  async (_proposal, o) => {
    let round = 0
    let last = null
    for (;;) {
      const fit = await agent(
        `Review one proposal for fit. Read ${ROOT}/proposals/${o.key}/proposal.md and proposal.json.

${REPO}

The question: ${A.question}

This is a proposal, not a software design. Judge the idea, not the implementation. Do not fault it for naming
no types, files or modules: it was told not to. A later stage designs those. Equally, do not let it get away
with an implementation claim it has not earned.

Judge:
- Is each pro grounded in something real, or asserted? Sort them.
- What cons has it missed? Reason about how gdtf actually plays and what this would do to it.
- Does it contradict itself, or contradict how the game works today?
- Is it a good fit for gdtf's battlescape at all?
- If it is a composite, is the division of labour real, and is the handover between layers actually specified?
  A composite that names three techniques and does not say how they agree is one technique and two hopes.

Check its claims about gdtf against the repo and docs/ where a claim is load-bearing. You are checking whether
it is true, not whether it is detailed.

Verdict GOOD when the argument stands up and the pros and cons are honest. NEEDS_WORK when it is fixable, and
then list exactly what must change in required. ABANDON only when the approach cannot work here, with the
reason.

A proposal carrying a recorded disagreement from a previous round has already answered you. Read it. If the
answer is sound, drop that point rather than repeating it.

Write to ${ROOT}/proposals/${o.key}/fit_check.json, overwriting any existing file.

${PLAIN}`,
        { label: `fit:${o.key} r${round}`, phase: 'Fit check', model: 'opus', effort: 'high', schema: FIT_SCHEMA },
      )
      last = fit
      if (!fit) return { key: o.key, abandoned: true, reason: 'fit check returned nothing', rounds: round }
      if (fit.verdict === 'GOOD') return { key: o.key, abandoned: false, rounds: round, fit }
      if (fit.verdict === 'ABANDON') return { key: o.key, abandoned: true, reason: fit.abandonReason, rounds: round }
      if (round >= MAX_ROUNDS) {
        return { key: o.key, abandoned: true, reason: `deadlocked after ${MAX_ROUNDS} fit rounds`, rounds: round, fit: last }
      }
      round += 1

      const rev = await agent(
        `Revise one proposal against its fit check.

Read ${ROOT}/proposals/${o.key}/proposal.md, proposal.json and fit_check.json.

Act on the feedback. Move an ungrounded pro onto real evidence or delete it. Add the missed cons. Resolve the
contradictions. Do everything listed in required.

You may disagree. If a criticism is wrong, say so in disagreements with the reason and the evidence, and leave
the proposal as it was on that point. A reviewer who is answered well is expected to drop the point. Do not
capitulate to a criticism you believe is wrong just to end the loop, and do not accept every point by reflex:
ten proposals that all agreed with their critics become ten of the same proposal.

If you now think this approach cannot work for gdtf, set abandoned true with the reason, and say so in the file
too. That is a legitimate outcome, not a failure.

Rewrite ${ROOT}/proposals/${o.key}/proposal.md and proposal.json.

${PLAIN}`,
        { label: `revise:${o.key} r${round}`, phase: 'Fit check', model: 'opus', effort: 'high', schema: REVISE_SCHEMA },
      )
      if (rev?.abandoned) return { key: o.key, abandoned: true, reason: rev.abandonReason, rounds: round }
    }
  },
)

survivors = proposalResults.filter(Boolean).filter(r => !r.abandoned).map(r => r.key)
abandoned = proposalResults.filter(Boolean).filter(r => r.abandoned)
log(`${survivors.length} proposals survived: ${survivors.join(', ')}`)

if (!survivors.length) {
  return {
    ticket: A.ticket,
    outcome: 'ALL_ABANDONED',
    rejected: abandoned.map(r => ({ key: r.key, reason: r.reason })),
    note: 'Start again from stage 1, telling the options agent these approaches were already rejected and why.',
    root: ROOT,
  }
}

} // end of the research half

// ---------------------------------------------------------------- judging

const JUDGE_SCHEMA = {
  type: 'object',
  properties: {
    lens: { type: 'string' },
    round: { type: 'number' },
    ranking: { type: 'array', items: { type: 'string' } },
    scores: { type: 'object', additionalProperties: { type: 'number' } },
    reasoning: { type: 'object', additionalProperties: { type: 'string' } },
    whatWouldChangeMyMind: { type: 'object', additionalProperties: { type: 'string' } },
    changedTop: { type: 'boolean' },
    whyChanged: { type: 'string' },
    whyNotChanged: { type: 'string' },
    rebuttals: {
      type: 'array',
      items: {
        type: 'object',
        properties: { toLens: { type: 'string' }, claim: { type: 'string' }, counter: { type: 'string' } },
        required: ['toLens', 'claim', 'counter'],
      },
    },
    conditionalOn: {
      type: 'array',
      items: {
        type: 'object',
        properties: { option: { type: 'string' }, condition: { type: 'string' } },
        required: ['option', 'condition'],
      },
    },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['lens', 'round', 'ranking', 'scores', 'reasoning', 'whatWouldChangeMyMind', 'changedTop', 'rebuttals', 'filesWritten'],
}

const JUDGE_RULES = `You are one of five judges, each reading through a different lens. You are expected to
disagree where your lens genuinely sees something the others do not, and you are expected to be persuadable
where it does not. Both halves matter. A judge who never moves is as useless as one who always folds.

Holding a position is something you earn, not a default. If another lens answers your objection, say so and
move. If you keep a ranking after being answered, your reasoning must say what specifically still stands and
why the answer did not reach it. Being hard to convince is not the same as being right, and a lens that treats
every round as a fight stops the whole panel from reaching an answer.

Rank every candidate. Score each out of ten. For each, give your reasoning and, in whatWouldChangeMyMind, the
specific thing that would move it up or down. Be concrete: name the fact, not a feeling.

If your agreement depends on something being true, do not silently rank on the assumption. Record it in
conditionalOn as the candidate plus the condition. A later stage acts on those.

You are never shown the running tally, and you must not ask for it or guess at it. Judge the arguments.`

async function judgeToConvergence({ phaseName, dir, subjects, lenses, brief, extraContext }) {
  const rounds = []
  let converged = false
  let deadlocked = false
  let stableFor = 0
  let prevTops = null

  for (let round = 0; ; round += 1) {
    const history = rounds.length
      ? `The previous round's judgements, all five lenses, are in ${dir}. Read every one of them. Then either
change your ranking or keep it, and say which and why. Answer the rebuttals aimed at your lens.

Previous round, verbatim:
${JSON.stringify(rounds[rounds.length - 1], null, 2)}`
      : `This is the first round. There is no previous judgement to read.`

    const votes = await parallel(lenses.map(lens => () => agent(
      `${JUDGE_RULES}

Your lens: ${lens.name}. ${lens.brief}

${REPO}

${brief}

${extraContext || ''}

The candidates, and where to read each in full:
${subjects.map(s => `- ${s.key}: ${s.path}`).join('\n')}

Read every candidate in full before ranking. Do not rank from its summary.

${history}

Write your judgement to ${dir}/round_${round}_${lens.key}.json.

${PLAIN}`,
      { label: `${phaseName === 'Judge proposals' ? 'judge' : 'djudge'}:${lens.key} r${round}`, phase: phaseName, model: 'fable', effort: 'high', schema: JUDGE_SCHEMA },
    )))

    const live = votes.filter(Boolean)
    if (!live.length) { deadlocked = true; break }
    rounds.push(live)

    const totals = {}
    for (const s of subjects) totals[s.key] = 0
    for (const v of live) {
      for (const [k, n] of Object.entries(v.scores || {})) {
        if (k in totals && typeof n === 'number') totals[k] += n
      }
    }
    const tops = live.map(v => v.ranking?.[0]).filter(Boolean)
    const unanimous = tops.length === live.length && new Set(tops).size === 1
    const noneMoved = live.every(v => v.changedTop === false)
    const sameAsLast = prevTops !== null && JSON.stringify(tops) === prevTops
    prevTops = JSON.stringify(tops)

    const ranked = Object.entries(totals).sort((a, b) => b[1] - a[1])
    log(`${phaseName} round ${round}: tops=${tops.join('/')} totals=${ranked.map(([k, n]) => `${k}:${n}`).join(' ')}`)

    await agent(
      `Write this scoring record to ${dir}/round_${round}_score.json, exactly as given, changing no number and
adding no commentary. You are a scribe. The score was computed by the workflow script, not by a model, and it
must stay that way.

${JSON.stringify({ round, totals, tops, unanimous, noneMoved, sameAsLast, ranked }, null, 2)}`,
      { label: `score r${round}`, phase: phaseName, model: 'sonnet' },
    )

    if (unanimous && noneMoved && sameAsLast) { stableFor += 1 } else { stableFor = 0 }
    if (unanimous && stableFor >= 1) { converged = true; break }
    if (round >= MAX_ROUNDS) { deadlocked = true; break }
  }

  const final = rounds[rounds.length - 1] || []
  const totals = {}
  for (const s of subjects) totals[s.key] = 0
  for (const v of final) {
    for (const [k, n] of Object.entries(v.scores || {})) {
      if (k in totals && typeof n === 'number') totals[k] += n
    }
  }
  const order = Object.entries(totals).sort((a, b) => b[1] - a[1]).map(([k]) => k)
  const conditions = final.flatMap(v => (v.conditionalOn || []).map(c => ({ lens: v.lens, ...c })))
  return { converged, deadlocked, rounds: rounds.length, totals, order, final, conditions }
}

if (!DESIGN_ONLY) {

phase('Judge proposals')

const PROPOSAL_LENSES = [
  { key: 'player', name: 'Player experience', brief: 'Judge what this produces on the board for someone playing. Interesting, readable, fair, varied.' },
  { key: 'authoring', name: 'Authoring and editor cost', brief: 'Judge who writes the content, how much, and what the content editor must grow. Authored content needs an editor screen and an editor MCP command.' },
  { key: 'integrity', name: 'Sim integrity', brief: 'Judge determinism, act parity between player and AI, TU accounting, and whether the sim stays the source of combat truth.' },
  { key: 'scale', name: 'Performance and scale', brief: 'Judge cost per frame and per turn as the roster and the board grow.' },
  { key: 'wrongness', name: 'Cost of being wrong', brief: 'Assume this turns out to be the wrong choice in six months. Judge what it costs to change, and what it has made hard to undo. Nobody else is looking at this, so raise it even when the others are enthusiastic. Concede it when someone shows the cost is smaller than you thought.' },
]

proposalJudging = await judgeToConvergence({
  phaseName: 'Judge proposals',
  dir: `${ROOT}/judgement`,
  subjects: survivors.map(k => ({ key: k, path: `${ROOT}/proposals/${k}/proposal.md` })),
  lenses: PROPOSAL_LENSES,
  brief: `The question: ${A.question}`,
})

topThree = proposalJudging.order.slice(0, 3)
log(`Top three proposals: ${topThree.join(', ')} (converged=${proposalJudging.converged}, deadlock=${proposalJudging.deadlocked})`)

} else {

const settled = await agent(
  `A proposal has already been decided and this run designs it. Make sure the proposal is on disk where the
design stages will read it.

The decided proposal is keyed \`${A.proposalKey}\`.

Check whether ${ROOT}/proposals/${A.proposalKey}/proposal.md already exists. If it does, read it, confirm it
describes the decided approach, and change nothing.

If it does not exist, write it, from whichever of these you have:
${A.proposalText ? 'The proposal text supplied to this run, below. Use it as the source of truth.' : 'The comments and description on Linear ticket ' + A.ticket + '. Load the Linear MCP tools through ToolSearch and read the ticket. Take the decided proposal from there.'}

${A.proposalText || ''}

Write it as a proposal, not a design: what the approach is, how it fits gdtf's battlescape, its pros and its
cons. Do not write types, file paths or module layouts. A later stage does that.

Write ${ROOT}/proposals/${A.proposalKey}/proposal.md and ${ROOT}/proposals/${A.proposalKey}/proposal.json.

${PLAIN}`,
  { label: `settled-proposal:${A.proposalKey}`, phase: 'Propose', model: 'opus', effort: 'high', schema: PROPOSAL_SCHEMA },
)

if (!settled) {
  return { ticket: A.ticket, outcome: 'FAILED', stage: 'Propose', reason: 'could not establish the settled proposal' }
}
survivors = [A.proposalKey]
topThree = [A.proposalKey]
log(`Designing the settled proposal ${A.proposalKey}`)

}

// ---------------------------------------------------------------- design

let designsReady = []
let designJudging = null
let conditionRounds = 0
let conditions = []
let winner = proposalJudging ? proposalJudging.order[0] : topThree[0]

if (!PROPOSAL_ONLY) {

phase('Design')

const DESIGN_SCHEMA = {
  type: 'object',
  properties: {
    key: { type: 'string' },
    overview: { type: 'string' },
    types: { type: 'array', items: { type: 'string' } },
    relations: { type: 'string' },
    pros: { type: 'array', items: { type: 'string' } },
    cons: { type: 'array', items: { type: 'string' } },
    justification: { type: 'string' },
    crateAndFiles: { type: 'array', items: { type: 'string' } },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['key', 'overview', 'types', 'relations', 'pros', 'cons', 'justification', 'crateAndFiles', 'filesWritten'],
}

const DESIGN_REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    key: { type: 'string' },
    verdict: { type: 'string', enum: ['AUDIT_OK', 'NEEDS_WORK'] },
    problems: { type: 'array', items: { type: 'string' } },
    ruleBreaches: { type: 'array', items: { type: 'string' } },
    required: { type: 'array', items: { type: 'string' } },
    filesWritten: { type: 'array', items: { type: 'string' } },
  },
  required: ['key', 'verdict', 'problems', 'ruleBreaches', 'filesWritten'],
}

const DESIGN_RULES = `The design is directional, not finished code. It does not have to compile. It does have to
be right about shape.

Binding: it is ECS first and uses Bevy entity relationships for anything that contains or attaches. It uses the
relationship API the pinned Bevy version actually provides, so check the version in Cargo.toml and check what
that version offers rather than writing from memory of another one. It uses no bare Rust type for a domain
value, per .claude/rules/no-bare-types.md: named newtypes that Deref. It respects the 400 line file cap in
.claude/rules/module-layout.md. It bans unwrap, expect, panic, todo and unimplemented. Every pub item is
documented.`

const designResults = await pipeline(
  topThree,
  k => agent(
    `Write a directional software design for one proposal.

${REPO}

The question: ${A.question}

Read ${ROOT}/proposals/${k}/proposal.md in full. Read the repo it names before designing against it.

${DESIGN_RULES}

${A.designBrief || ''}

Write, in this order: an overview paragraph a reader can understand before the detail; the types, written out
with fields; how containment or attachment uses relationships; which crate and which files it lives in; the
pros; the cons; and why this is a good fit.

Write the prose to ${ROOT}/proposals/${k}/design.md and the structured form to ${ROOT}/proposals/${k}/design.json.

${PLAIN}`,
    { label: `design:${k}`, phase: 'Design', model: 'opus', effort: 'high', schema: DESIGN_SCHEMA },
  ),
  async (_d, k) => {
    let round = 0
    for (;;) {
      const rev = await agent(
        `Review one software design. Read ${ROOT}/proposals/${k}/design.md and design.json, and the proposal beside
them.

${REPO}

${DESIGN_RULES}

Judge it as an engineer who will have to maintain it: does the shape work, does it fit the crate graph and the
existing idiom, what breaks under save and load, what breaks when content changes underneath it, and what does
it make hard.

List every rule breach separately in ruleBreaches, quoting the rule.

Verdict AUDIT_OK only when nothing remains. Otherwise NEEDS_WORK with exactly what must change in required.

Write to ${ROOT}/proposals/${k}/design_review.json, overwriting any existing file.

${PLAIN}`,
        { label: `dreview:${k} r${round}`, phase: 'Review design', model: 'opus', effort: 'high', schema: DESIGN_REVIEW_SCHEMA },
      )
      if (!rev) return { key: k, ok: false, rounds: round, reason: 'review returned nothing' }
      if (rev.verdict === 'AUDIT_OK') return { key: k, ok: true, rounds: round }
      if (round >= MAX_ROUNDS) return { key: k, ok: false, rounds: round, reason: `deadlocked after ${MAX_ROUNDS} review rounds`, review: rev }
      round += 1

      await agent(
        `Revise one software design against its review.

Read ${ROOT}/proposals/${k}/design.md, design.json and design_review.json.

Do everything in required. Fix every rule breach.

You may disagree with a point, and if you do, say so in the design file itself as one short paragraph naming
the reason and the evidence, and leave that part as it was. Do not accept a criticism you believe is wrong
just to end the loop.

Rewrite ${ROOT}/proposals/${k}/design.md and design.json.

${PLAIN}`,
        { label: `drevise:${k} r${round}`, phase: 'Review design', model: 'opus', effort: 'high', schema: DESIGN_SCHEMA },
      )
    }
  },
)

designsReady = designResults.filter(Boolean).map(r => r.key)
log(`Designs ready: ${designsReady.join(', ')}`)

phase('Judge designs')

const DESIGN_LENSES = [
  { key: 'ecs', name: 'ECS idiom and relations', brief: 'Judge whether this is genuinely ECS-shaped, whether it uses relationships properly, and whether it fights the engine anywhere.' },
  { key: 'testability', name: 'Testability', brief: 'Judge what goes red when this breaks. A design nothing can fail on is not testable, however clean it reads.' },
  { key: 'migration', name: 'Migration and persistence', brief: 'Judge save and load round-tripping, and what happens when authored content changes under saved state.' },
  { key: 'layout', name: 'Module layout and typing', brief: 'Judge the crate and file placement, the 400 line cap, and whether every domain value has a named type rather than a bare one.' },
  { key: 'rigidity', name: 'What it makes hard', brief: 'Judge what this design resists. Name the requirement that would be expensive under it. Nobody else is looking at this, so raise it even when the others like the design. Elegance and flexibility are different things, and a clean design can still be rigid. Concede when someone shows the rigidity is not real.' },
]

designJudging = await judgeToConvergence({
  phaseName: 'Judge designs',
  dir: `${ROOT}/design_judgement`,
  subjects: designsReady.map(k => ({ key: k, path: `${ROOT}/proposals/${k}/design.md` })),
  lenses: DESIGN_LENSES,
  brief: `The question: ${A.question}\n\nThese are directional designs, not finished code. Judge the shape, not the syntax. Do not mark a design down for being incomplete in a way it was asked to be.`,
  extraContext: `You are judging designs only. An earlier round judged the proposals these came from, and you are
deliberately not being shown its verdicts or its scores, so that you rank these designs on their own merits.
Do not go looking for them.`,
})

phase('Close conditions')

conditions = designJudging.conditions
winner = designJudging.order[0]

while (conditions.length && conditionRounds < MAX_ROUNDS) {
  conditionRounds += 1
  log(`Closing ${conditions.length} conditions on ${winner}, round ${conditionRounds}`)

  await agent(
    `The judges agreed on this design, but conditionally. Satisfy the conditions.

Read ${ROOT}/proposals/${winner}/design.md and design.json.

The conditions, each from a named lens:
${JSON.stringify(conditions, null, 2)}

For each: change the design so the condition holds, or state in the design file why it cannot hold and what
that costs. Do not leave one unaddressed and do not restate it as future work.

${DESIGN_RULES}

Rewrite ${ROOT}/proposals/${winner}/design.md and design.json.

${PLAIN}`,
    { label: `close-conditions r${conditionRounds}`, phase: 'Close conditions', model: 'opus', effort: 'high', schema: DESIGN_SCHEMA },
  )

  const recheck = await agent(
    `Check whether a revised design satisfies the conditions the judges attached to it.

Read ${ROOT}/proposals/${winner}/design.md.

The conditions:
${JSON.stringify(conditions, null, 2)}

For each, say whether it now holds, and quote the part of the design that settles it. A condition answered by
a promise rather than by the design does not hold.

Verdict AUDIT_OK only when every condition holds. Otherwise NEEDS_WORK listing what remains in required.

Write to ${ROOT}/proposals/${winner}/condition_check.json.

${PLAIN}`,
    { label: `condition-check r${conditionRounds}`, phase: 'Close conditions', model: 'opus', effort: 'high', schema: DESIGN_REVIEW_SCHEMA },
  )

  if (!recheck || recheck.verdict === 'AUDIT_OK') break
  conditions = (recheck.required || []).map(r => ({ lens: 'recheck', option: winner, condition: r }))
}

}

// ---------------------------------------------------------------- publish

phase('Publish')

const PUBLISH_SCHEMA = {
  type: 'object',
  properties: {
    ticket: { type: 'string' },
    proposalCommentId: { type: 'string' },
    designCommentId: { type: 'string' },
    questionsCommentId: { type: ['string', 'null'] },
    labelsAfter: { type: 'array', items: { type: 'string' } },
    descriptionUpdated: { type: 'boolean' },
    notes: { type: 'string' },
  },
  required: ['ticket', 'proposalCommentId', 'designCommentId', 'labelsAfter', 'descriptionUpdated', 'notes'],
}

const published = await agent(
  PROPOSAL_ONLY
    ? `Publish a finished research result to Linear ticket ${A.ticket} on the GDTF board. This run produced a
chosen proposal and no software design, deliberately.

The winning option is ${winner}. Read ${ROOT}/proposals/${winner}/proposal.md in full and post it verbatim as a
comment. You are the courier, not an editor. Do not summarise, trim, reorder or improve it.

Head the comment with \`**[research-and-design]**\` on its own line and nothing above it, then one line saying
it is the proposal from a judged research workflow, how many options were compared, that no software design was
produced, and that the owner has not ruled on it.

Linear's save_comment drops the first two or three characters of a table cell that opens with a backtick or a
quote. If the file contains a markdown table, convert that table to bullets. If it does not, change nothing.

If the comment exceeds Linear's size limit, split it across consecutive comments, each headed the same way with
a line saying it continues the previous one. Split at a section boundary, never inside a code block. Never trim
to fit.

If the proposal lists questions only the owner can answer, post a second comment holding just those, numbered,
each in one or two plain sentences answerable without reading the proposal, and add the \`Needs User Input\`
label. If there are none, post no second comment and add no such label.

Then update the ticket:
- Remove \`Needs Research\`. It is answered.
- KEEP \`Needs Design\` if it is present, and add it if it is not. No software design was produced by this run,
  so the ticket still needs one. This is the difference from a full run and it matters.
- Add \`Needs Splitting\` if it is not already there.
- Labels need \`team: GDTF\` on the query or you get only the workspace labels back with a \`hasNextPage: false\`
  that reads as a complete list and is not.
- Edit the description so its prose points at the proposal comment by name. Do not paste the proposal into the
  description, and do not delete what the description already says about the problem.

Set designCommentId to the string "none" in your return value, since there is no design comment.

Do not change status. Do not file any ticket.

Load the Linear MCP tools through ToolSearch if needed. Re-fetch afterwards and confirm what landed.

${PLAIN}`
    : `Publish a finished research and design result to Linear ticket ${A.ticket} on the GDTF board.

The winning option is ${winner}. Read these two files in full and post them verbatim, as two separate comments:
- ${ROOT}/proposals/${winner}/proposal.md
- ${ROOT}/proposals/${winner}/design.md

You are the courier, not an editor. Do not summarise, trim, reorder or improve either. Code blocks must survive
whole.

Head each comment with \`**[research-and-design]**\` on its own line and nothing above it, then one line saying
which of the two it is, that it is the output of a judged research workflow, how many options were compared,
and that the owner has not ruled on it.

Linear's save_comment drops the first two or three characters of a table cell that opens with a backtick or a
quote. If a file contains a markdown table, convert that table to bullets. If it does not, change nothing.
Check before converting.

If a comment exceeds Linear's size limit, split it across consecutive comments, each headed the same way with a
line saying it continues the previous one. Split at a section boundary, never inside a code block. Never trim
to fit.

If either file lists questions only the owner can answer, post a third comment holding just those, numbered,
each in one or two plain sentences answerable without reading the other comments, and add the \`Needs User
Input\` label. If there are none, post no third comment and add no such label.

Then update the ticket:
- Remove \`Needs Research\` and \`Needs Design\` if present. Both are now answered.
- Add \`Needs Splitting\` if it is not already there.
- Change nothing else. Labels need \`team: GDTF\` on the query or you get only the workspace labels back with a
  \`hasNextPage: false\` that reads as a complete list and is not.
- Edit the description so its prose points at the two comments by name, saying the proposal and the software
  design now live there. Do not paste either into the description, and do not delete what the description
  already says about the problem.

Do not change status. Do not file any ticket.

Load the Linear MCP tools through ToolSearch if needed. Re-fetch afterwards and confirm what landed.

${PLAIN}`,
  { label: `publish:${A.ticket}`, phase: 'Publish', model: 'sonnet', schema: PUBLISH_SCHEMA },
)

return {
  ticket: A.ticket,
  outcome: 'COMPLETE',
  mode: PROPOSAL_ONLY ? 'PROPOSAL_ONLY' : 'PROPOSAL_AND_DESIGN',
  root: ROOT,
  winner,
  optionsConsidered: optionList.map(o => o.key),
  optionsDeadlocked,
  abandonedProposals: abandoned.map(r => ({ key: r.key, reason: r.reason })),
  survivors,
  proposalJudging: {
    converged: proposalJudging.converged,
    deadlocked: proposalJudging.deadlocked,
    rounds: proposalJudging.rounds,
    totals: proposalJudging.totals,
    order: proposalJudging.order,
  },
  topThree: PROPOSAL_ONLY ? [] : topThree,
  designsReady,
  designJudging: designJudging && {
    converged: designJudging.converged,
    deadlocked: designJudging.deadlocked,
    rounds: designJudging.rounds,
    totals: designJudging.totals,
    order: designJudging.order,
  },
  conditionRounds,
  conditionsRemaining: conditions,
  files: {
    options: `${ROOT}/options.json`,
    winningProposal: `${ROOT}/proposals/${winner}/proposal.md`,
    winningDesign: PROPOSAL_ONLY ? null : `${ROOT}/proposals/${winner}/design.md`,
    proposalDeliberations: `${ROOT}/judgement`,
    designDeliberations: PROPOSAL_ONLY ? null : `${ROOT}/design_judgement`,
  },
  published,
}

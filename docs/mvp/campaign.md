# Campaign Layer (Deferred Past MVP)

Everything here is **deferred** — it is the wrapper around the proven core loop, not the loop itself (see [mvp.md](mvp.md)). Captured now so the decisions aren't lost, **not** scheduled for v0.

## Geoscape — hex, procgen per campaign

- **Hex grid.** In a turf war, adjacency *is* the conflict — who borders whom decides who fights, who can expand, who holds a grudge. Hex gives six clean equal neighbors with no diagonal-adjacency ambiguity. Square's 8-vs-4 neighbor mess is a tax for nothing here.
- **This is a Necromunda underhive turf map (Civ-lite), NOT an XCOM globe.** No literal planet/globe.
- **Procgen every campaign.** Different turf politics each run is the situation generator operating at the *strategic* scale — the whole thesis, applied one level up.

## Grudges with memory — the moat

The single most differentiating system. XCOM doesn't do this; Necromunda only gestures at it.

- Gangs (and gangers) **remember**: who crippled whom, who they owe, who they want dead.
- Seeded by the Injury table and the who-killed/wounded-whom log from the MVP (see [wounds-and-roster.md](../combat/wounds-and-roster.md)) — that log is the raw material *(neither the named injury tables nor the kill log is built yet; in-battle per-hit wound severity is what exists today)*.
- A rivalry system with memory drives emergent flashpoints, especially when layered onto hex adjacency (a bordering gang you have a grudge against = a powder keg).
- It's **data and rules, not art** — squarely the project's strengths.

**TBD (design):** the grudge data model — what's remembered, how it decays/escalates, how it influences mission generation and AI behavior.

## Turf & economy

- **Turf → income.** Controlled turf generates resources.
- **Captures → rescue missions.** Captured gangers create follow-up situations (rescue, ransom, defection).
- Mission generation parameterized by objective / terrain / opposing gang.

## Rubber-band / underdog mechanics

Comeback arcs are part of what keeps a long campaign interesting — a gang that's been beaten down needs a path back, or the campaign flattens into a runaway leader. **TBD (design):** the specific mechanic (catch-up income, underdog bonuses, etc.). Tune carefully — too strong and it erases the stakes the permadeath/wound layer creates.

## Spatial RPS connection

The two-paradox matchup math ([matchup.md](../combat/matchup.md)) has a cellular-automaton / spatial framing: turf control where adjacent gang "types" interact under tournament rules generates shifting front lines. Whether to lean into this at the geoscape scale is **TBD (design)** — noted as a promising direction, not a commitment.

## Sequencing

Build order after the MVP loop is proven: **(1)** a minimal geoscape that strings missions together with persistent roster, **(2)** turf/income, **(3)** grudges surfaced from the existing log, **(4)** captures/rescues, **(5)** procgen + rubber-band tuning. Each step should be independently fun before the next is added.

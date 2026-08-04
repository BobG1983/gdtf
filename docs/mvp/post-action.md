# Post-action — injury carry and advancement (MVP)

Design for the fight → consequences → next fight loop. Parent: post-action epic under Battlescape.

**Status:**

- **GTW-678 injury carry — Accepted** (in-battle injury table already defined; no second post-action roll).
- **GTW-679 advancement — proposed** (use-bump cap amended below). Do not implement until Accepted.

Related built systems:

- Named injuries already roll **per hit in battle** and land on `InflictedInjuries` (see [wounds-and-roster.md](../combat/wounds-and-roster.md)). That **is** the injury roll — location × severity × source weighting is already specified and live.
- Attributes improve by use; XP buys skills (see [stats.md](../combat/stats.md)) — neither campaign track is built yet.

---

## 1. Injury after battle (GTW-678) — Accepted

### Conflict with older MVP text

`mvp.md` used to say the post-action screen **rolls** survivors on the Injury table. Combat already rolls the named injury table **during** the fight. A second full table roll would double-punish and fight the ledger.

### Decision

**No post-action re-roll of the injury table for MVP.** The in-battle roll is the definition.

| Who | What happens at battle end |
| --- | --- |
| **Dead** | Stay dead on the roster. No roll. |
| **Alive with `InflictedInjuries`** (standing or Downed) | **Carry the ledger forward.** That is the scar. No new named injury is drawn from the table. |
| **Downed and alive when the battle ends** | **Recover** into the roster with existing ledger injuries (capture deferred past MVP — see [campaign.md](campaign.md)). |

Answers to the ticket questions:

1. **Who rolls?** Nobody re-rolls the named injury table after battle. In-battle rolls already did that job.
2. **What feeds the roll?** N/A for post-action. In battle: location × severity × source weighting (already built).
3. **Stack with in-battle injuries?** There is no second roll to stack. Carry = the same ledger. Later battles add more entries to the same ganger over a campaign.

### What post-action *does* build (implement tickets, not this design doc)

1. Snapshot each surviving player ganger’s `InflictedInjuries` (and identity) into **BattleResults** / roster write-back.
2. Show those scars on the post-action screen (names + short injury text).
3. Seed the next battle from the scarred roster so modifiers and missing hands still apply.

### Explicitly not MVP

| Deferred | Note |
| --- | --- |
| Second “serious injury” table for Out-of-Action (classic Necromunda) | Rejected for v0; ledger is enough |
| Capture / rescue of Downed on lost turf | Campaign layer |
| Critical infection / post-battle death dial | Still TBD in wounds doc; off for MVP |
| Acute → residual downtime calendar | Bench rules stay TBD; for v0 residuals = full ledger until a later recovery pass |

### Build order after Accept

Data first (no UI): BattleResults + kill log + roster fold. Then screen copy. Headless tests: seed a ganger with a known ledger, end battle, assert the same injuries on the campaign roster resource.

---

## 2. XP and minimal advancement (GTW-679)

### Canon (unchanged)

Two tracks:

1. **Use-based attributes** — exercising a capability trains the attribute behind it (Xenonauts-style).
2. **XP** — a separate bank that **buys skills**, never raw attribute points.

### Accrual events (per player ganger, once at outcome latch)

| Event | When it counts | Default XP (tunable) |
| --- | --- | --- |
| **Participation** | Deployed and took at least one turn (or one spent TU) this battle | 2 |
| **Kill credit** | Attributed kill in the who-killed-whom log | +3 per kill |
| **Survival** | Alive at outcome (standing or Downed-recovered) | +1 |

No XP for taking damage or for enemy deaths you did not cause. Magnitudes live in tuning data (e.g. `advancement.tuning.ron`); tests assert **properties** (kill gives more than participation alone, dead get no survival), not hard-coded totals.

Enemy gangers: no XP track for MVP.

### Use-based attributes (minimal application)

Count **UsageTally** during battle from existing act messages (shots, melee swings, moves, etc. — implement under the UsageTally ticket).

At outcome latch, **once per battle**, sum all sources into a per-attribute bump, then clamp.

#### Default use (ordinary fight work)

| Tally signal (example) | Default bump |
| --- | --- |
| Ranged attacks resolved ≥ 1 | Aim +1 |
| Melee attacks resolved ≥ 1 | Strength +1 |
| Move acts / cells entered ≥ N (tunable, default 5) | Speed +1 |
| Ended battle conscious after taking HP damage | Grit +1 |

Default from use is **+1** for that attribute when the signal fires. Meeting the bar harder does **not** by itself stack more default bumps (one default grant per attribute per battle).

#### Special extras

Other sources may add **extra +1** (or more, if that source says so) on top of the default:

- Content / skills / injuries / mission rewards that explicitly grant an attribute train bonus
- Future “feat” or objective tags (e.g. first blood, last stand) when those systems exist

Each special is data-authored; the apply step just sums them.

#### Cap

**Hard ceiling: +3 to any one attribute from all sources in one battle.**  
Sum(default use + specials), then `min(sum, 3)`. Unused headroom is fine (many fights stay at +1).

Rules:

- Only **living** gangers (Dead get no bumps).
- Bumps write to **base attributes** on the campaign roster, then re-derive combat stats for the next fight.
- No mid-battle permanent attribute growth.

Other attributes (Toughness, Reflexes, Cool, Luck) stay flat until a later use map expands. Specials may still target them once content exists; same +3 cap.

### XP bank vs skills

- Accrued XP sums onto the ganger’s **skill XP bank**.
- **Skill purchase UI and skill content are not MVP** unless a separate ticket lands them. Post-action shows the bank number so the track is visible.
- XP never converts into free attribute points.

### When

All of the above runs **once** when the battle outcome latches (same moment as BattleResults), before or as the post-action screen opens. Not per act.

### Build order after Accept

1. UsageTally from act messages.
2. Kill log (for kill XP and grudges later).
3. Advancement apply function (pure sim, seeded tests) — including special bonus hooks + +3 clamp.
4. Wire into BattleResults / roster fold.
5. Post-action UI readout.

---

## 3. Screen (shared)

Post-action UI shows, for the player roster:

- Outcome (win / lose).
- Per ganger: alive/dead, carried injuries (short text), XP gained this battle, use bumps this battle, running skill XP.
- Continue → roster write-back → next battle or menu (mission chaining is a separate ticket).

Placeholder outcome + Continue stays until this data exists.

---

## 4. Open questions for sign-off

**Injury (678):** Accepted as written — carry ledger only; in-battle roll is the definition.

Remaining (679 / shared):

1. **Downed:** Recover with ledger only for MVP (no capture)?
2. **XP mix:** Participation + kill + survival OK?
3. **Default XP magnitudes:** 2 / 3 / 1 OK as tuning starting points?
4. **Use map:** Aim / Strength / Speed / Grit as the default-use set for v0 OK?
5. **Use cap:** default +1 from use, specials can add, hard max **+3** per attribute per battle OK?
6. **Skills:** XP bank visible but no purchase UI in v0 OK?

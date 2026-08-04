# Post-action — injury carry and advancement (MVP)

Design for the fight → consequences → next fight loop. Parent: post-action epic under Battlescape.

**Status:**

- **GTW-678 injury carry — Accepted** (in-battle injury table already defined; no second post-action roll).
- **GTW-679 advancement — proposed** (stress / crit / survive / MVP training). Do not implement until Accepted.

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

1. **Use-based attributes** — exercising a capability trains the attribute behind it (Xenonauts-style). Growth comes from **using**, **stressing**, and **surviving** that capability — not from a kill scoreboard.
2. **XP** — a separate bank that **buys skills**, never raw attribute points.

### Accrual events (per player ganger, once at outcome latch)

| Event | When it counts | Default XP (tunable) |
| --- | --- | --- |
| **Participation** | Deployed and took at least one turn (or one spent TU) this battle | 2 |
| **Kill credit** | Attributed kill in the who-killed-whom log | +3 per kill |
| **Survival** | Alive at outcome (standing or Downed-recovered) | +1 |

No XP for taking damage or for enemy deaths you did not cause. Magnitudes live in tuning data (e.g. `advancement.tuning.ron`); tests assert **properties**, not hard-coded totals.

Enemy gangers: no XP track for MVP.

### Use-based attributes — three layers

At outcome latch, for each living player ganger, sum layers per attribute, then clamp.

**Hard ceiling: +3 per attribute per battle** from all layers combined.

```
bump = min(3, default_use + stress + crit_success + survive_bad + mvp_share)
```

Each named grant fires **at most once per attribute per battle** (unless the row says otherwise). Dead gangers get nothing.

---

#### Layer A — Default use (+1)

Ordinary fight work. Signal met → **+1** to that attribute. Harder use does not stack more default.

| Signal | Attribute |
| --- | --- |
| ≥1 ranged attack resolved | Aim |
| ≥1 melee attack resolved | Strength |
| Cells entered / move acts ≥ N (tunable, default 5) | Speed |
| Took any HP damage and still conscious at some point after | Grit |

---

#### Layer B — Stress / failure (+1)

**Train by failing or loading the stat.** These fire when the ganger was *tested*, not when they looked good on the scoreboard.

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Missed shot** | ≥1 ranged attack resolved that did **not** connect (no hit on any combatant / intended silhouette fail — exact predicate = same “miss” the combat log already records) | +1 | Aim |
| **Missed melee** | ≥1 melee attack that failed to connect | +1 | Strength |
| **Got hurt** | Took ≥1 non-graze wound (any severity) | +1 | Toughness |
| **Morale stress** | Lost any Morale this battle (when Morale is live; until then: took a suppression / Bottle hit if those signals exist, else defer this row) | +1 | Cool |
| **Overloaded** | Spent any turn over carrying capacity / Strength shortfall (encumbrance — when built; until then defer) | +1 | Strength |
| **Pushed empty** | Ended ≥1 of own turns at 0 TU after having spent TU that turn (fought to the last unit) | +1 | Grit |

Stress does **not** require a kill. A lousy fight that you survive still trains.

---

#### Layer C — Critical success (+1)

**One clean win under that stat** — the upside twin of stress. Same once-per-battle rule.

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Ranged kill** | ≥1 attributed kill with a ranged weapon | +1 | Aim |
| **Melee kill** | ≥1 attributed kill with melee / unarmed | +1 | Strength |
| **Critical wounder** | Inflicted ≥1 Critical or Fatal wound on an enemy (injury/wound log) | +1 | Aim if ranged source, Strength if melee (both kinds → one grant each) |
| **Clutch stabilize** | Successfully stabilized a Downed ally | +1 | Cool |
| **Clean execute** | Successfully executed a Downed enemy | +1 | Cool |

Kill XP still tracks kills separately. Crit-success is the attribute bump, not a second XP table.

---

#### Layer D — Survive a bad wound (+1)

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Walked it off** | Ended the battle alive after taking a **Major** or **Critical** injury this battle (ledger gained that tier) | +1 | Grit |
| **Back from the brink** | Was **Downed** at least once and still alive at outcome | +1 | Grit |

These stack with each other and with default Grit / stress, then hit the +3 cap.

---

#### Layer E — MVP (+1)

**One player ganger per battle** may earn MVP.

| Rule | Detail |
| --- | --- |
| Who | Highest attributed kill count among living player gangers at outcome. Tie → most non-graze wounds inflicted. Still tied → lowest entity/id (deterministic). |
| Reward | **+1** to one attribute: the attribute that received the largest pre-MVP bump this battle; if none, **Cool**. |
| None | If no player ganger got a kill and no wounds inflicted, **no MVP**. |

MVP is a single +1, not a free +3.

---

### Worked examples

| Fight | Layers that fire | Result (before cap) |
| --- | --- | --- |
| Shot a lot, missed often, no kills | Aim use +1, Missed shot +1 | **Aim +2** |
| Shot, one kill, also missed | Aim use +1, miss +1, ranged kill +1 | **Aim +3** (cap) |
| Took a Major arm injury, lived | Grit use (if HP lost) +1, Got hurt → Toughness +1, Walked it off → Grit +1 | **Toughness +1, Grit +2** |
| Downed, stabilized ally, recovered | Brink +1 Grit, Stabilize +1 Cool, … | Grit/Cool as listed |
| Squad ace with 3 kills | Use + kills + MVP | Aim (or Cool) up to **+3** |

---

### Explicitly not training (v0)

Reload, stance change, open door, enter emplacement, empty grenade throw, panicking enemies by presence alone, raw damage totals without miss/hit/wound events.

---

### XP bank vs skills

- Accrued XP sums onto the ganger’s **skill XP bank**.
- **Skill purchase UI and skill content are not MVP** unless a separate ticket lands them. Post-action shows the bank number so the track is visible.
- XP never converts into free attribute points.

### When

All of the above runs **once** when the battle outcome latches (same moment as BattleResults), before or as the post-action screen opens. Not per act.

### Build order after Accept

1. UsageTally + miss/hit/wound/kill/Downed signals from act messages (shared log).
2. Advancement apply (pure sim, seeded tests) — five layers + +3 clamp + deterministic MVP.
3. Wire into BattleResults / roster fold.
4. Post-action UI: bumps + which specials fired (short names).

Dependencies that gate individual rows: Morale stress needs Morale live; Overloaded needs encumbrance. Ship the rest first; those two rows stay coded but inactive until their systems exist.

---

## 3. Screen (shared)

Post-action UI shows, for the player roster:

- Outcome (win / lose).
- Per ganger: alive/dead, carried injuries, XP gained, attribute bumps with special names, MVP badge if any, running skill XP.
- Continue → roster write-back → next battle or menu (mission chaining is a separate ticket).

Placeholder outcome + Continue stays until this data exists.

---

## 4. Open questions for sign-off

**Injury (678):** Accepted.

Remaining (679):

1. **Downed:** Recover with ledger only for MVP (no capture)?
2. **XP mix:** Participation + kill + survival at 2 / 3 / 1 OK?
3. **Five-layer training** (use / stress / crit success / survive bad / MVP) + hard max +3 OK?
4. **Stress rows:** miss shot, miss melee, got hurt, morale stress, overloaded, pushed empty — drop/add any?
5. **Crit rows:** ranged/melee kill, critical wounder, stabilize, execute — OK?
6. **Survive:** Major/Critical lived +1 Grit; was Downed and lived +1 Grit — OK?
7. **MVP:** auto by kills (tie wounds, then id); +1 to strongest trained attr else Cool — OK?
8. **Skills:** XP bank visible, no purchase UI in v0 OK?

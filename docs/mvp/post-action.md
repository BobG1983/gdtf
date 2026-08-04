# Post-action — injury carry and advancement (MVP)

Design for the fight → consequences → next fight loop. Parent: post-action epic under Battlescape.

**Status:**

- **Injury carry — Accepted** (in-battle injury table already defined; no second post-action roll).
- **Advancement — proposed** (training vs XP split + combined specials). Do not implement until Accepted.

Related built systems:

- Named injuries already roll **per hit in battle** and land on `InflictedInjuries` (see [wounds-and-roster.md](../combat/wounds-and-roster.md)). That **is** the injury roll — location × severity × source weighting is already specified and live.
- Advancement tracks: see [stats.md](../combat/stats.md) — **not built yet**.

---

## 1. Injury after battle — Accepted

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

## 2. Advancement — proposed

### Two tracks — hard split

| Track | What it changes | How you earn it |
| --- | --- | --- |
| **Training** | **Base attributes** only | Use, stress/fail, feats, **surviving damage / morale loss / Downed** |
| **XP** | **Skill bank only** | Participation, kills, **survival (alive at end)**, **MVP** |

**XP never buys attribute points. Training never grants skills.**  
**MVP and mere survival are XP, not attribute training.**  
**Getting hurt, losing morale, and walking away from serious wounds are training (stats).**

Skills cost XP when a skill system exists; until then the bank just accrues and shows on the post-action screen.

### XP accrual (skill bank only)

Per player ganger, once at outcome latch:

| Event | When it counts | Default XP (tunable) |
| --- | --- | --- |
| **Participation** | Deployed and took at least one turn (or one spent TU) this battle | 2 |
| **Kill credit** | Attributed kill in the who-killed-whom log | +3 per kill |
| **Survival** | Alive at outcome (standing or Downed-recovered) | +1 |
| **MVP** | Selected as battle MVP (see below) | +5 |

No XP for taking damage. Magnitudes live in tuning data; tests assert **properties**, not hard-coded totals.

Enemy gangers: no XP track for v0.

#### MVP (XP only)

**One player ganger per battle** may earn MVP.

| Rule | Detail |
| --- | --- |
| Who | Highest attributed kill count among living player gangers at outcome. Tie → most non-graze wounds inflicted. Still tied → lowest stable id (deterministic). |
| Reward | **+5 XP** (tunable). No attribute bump. |
| None | If no player ganger got a kill and no wounds inflicted, **no MVP**. |

### Training — attribute growth

At outcome latch, for each living player ganger, sum every training grant that fired, **per attribute**, then clamp.

**Hard ceiling: +3 per attribute per battle.**

```
bump = min(3, default_use + stress + feats + survive_damage)
```

Each named grant fires **at most once per ganger per battle**. Dead gangers get no training.

Training writes to **base attributes** on the campaign roster, then re-derive combat stats for the next fight. No mid-battle permanent attribute growth.

---

#### Layer A — Default use (+1)

Ordinary fight work. Signal met → **+1**. Harder use does not stack more default.

| Signal | Attribute |
| --- | --- |
| ≥1 ranged attack resolved | Aim |
| ≥1 melee attack resolved | Strength |
| Cells entered / move acts ≥ N (tunable, default 5) | Speed |

*(Took HP / “still standing” is not default use — it lives under stress / survive-damage so damage trains Toughness/Grit, not a free Grit for existing.)*

---

#### Layer B — Stress / failure (+1)

Train when the stat was **tested or failed** — including taking damage and morale hits.

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Missed shot** | ≥1 ranged attack that did not connect | +1 | Aim |
| **Missed melee** | ≥1 melee attack that failed to connect | +1 | Strength |
| **Got hurt** | Took ≥1 non-graze wound (any severity) | +1 | Toughness |
| **HP stress** | Lost any HP this battle (even graze-only chip) | +1 | Grit |
| **Morale stress** | Lost any Morale this battle (when Morale is live; until then: suppression / Bottle hit if those signals exist, else defer) | +1 | Cool |
| **Overloaded** | Spent any turn over carrying capacity / Strength shortfall (when encumbrance exists; else defer) | +1 | Strength |
| **Pushed empty** | Ended ≥1 of own turns at 0 TU after spending TU that turn | +1 | Grit |

---

#### Layer C — Feats / critical success (+1, unless noted)

Clean wins and named battle feats. (Not MVP — MVP is XP.)

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Ranged kill** | ≥1 attributed kill with a ranged weapon | +1 | Aim |
| **Melee kill** | ≥1 attributed kill with melee / unarmed | +1 | Strength |
| **Multi-kill** | ≥2 attributed kills (any weapon) | +1 | Cool |
| **Critical wounder** | Inflicted ≥1 Critical or Fatal wound on an enemy | +1 | Aim if ranged source, Strength if melee (both kinds → one grant each) |
| **Clutch stabilize** | Successfully stabilized a Downed ally | +1 | Cool |
| **Clean execute** | Successfully executed a Downed enemy | +1 | Cool |
| **Shove finish** | Shoved an enemy who then died from the fall / shove outcome this battle | +1 | Strength |
| **Last of the squad** | Only living player ganger at outcome (others dead or never deployed) **and** battle was won | +2 | Cool |

---

#### Layer D — Survive damage / bad shape (+1 / +2)

**Stats for living through punishment** — not a participation trophy. Mere “alive at end with no drama” is XP survival only.

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Walked it off** | Alive at outcome after taking a **Major** or **Critical** injury this battle | +1 | Grit |
| **Back from the brink** | Was **Downed** at least once and still alive at outcome | +2 | Grit |
| **Held their nerve** | Lost Morale (or Bottle) this battle **and** did not bottle out / flee (when Morale is live; else defer) | +1 | Cool |

---

### Worked examples

| Fight | Training | XP |
| --- | --- | --- |
| Shot a lot, missed, no kills, walked out unhurt | Aim use +1, Missed shot +1 → **Aim +2** | Participation + survival |
| Shot, missed, one kill, MVP | Aim up to +3 | Participation + kill + survival + **MVP +5** |
| Major injury, Downed, recovered | Got hurt Toughness +1; HP stress Grit +1; Walked it off +1; Brink +2 → **Toughness +1**, **Grit +3** (cap) | Participation + survival (no MVP unless kills) |
| Lost morale, held | Morale stress Cool +1; Held their nerve Cool +1 | Participation + survival |

---

### Explicitly not training (v0)

Reload, stance change, open door, enter emplacement, empty grenade throw, panicking enemies by presence alone, raw damage totals without miss/hit/wound/kill events, **mere survival without damage/morale drama**, **MVP**.

---

### Skills (XP spend)

- Skill purchase UI and skill content are **not v0** unless a separate ticket lands them.
- Post-action still shows the **skill XP bank**.
- When skills exist: spend XP → learn skill; attributes still only move via training.

### When

Training + XP both apply **once** when the battle outcome latches (same moment as BattleResults), before or as the post-action screen opens. Not per act.

### Build order after Accept

1. UsageTally + miss/hit/wound/kill/Downed/shove/morale signals from act messages.
2. Apply functions (pure sim, seeded tests):
   - `apply_training` — layers A–D + +3 clamp → base attributes
   - `apply_xp` — participation / kills / survival / MVP → skill bank only
3. Wire into BattleResults / roster fold.
4. Post-action UI: training bumps + special names, XP lines (incl. MVP), bank total.

Dependencies that gate individual rows: Morale stress / Held their nerve need Morale live; Overloaded needs encumbrance. Ship the rest first; those rows stay defined but inactive until their systems exist.

---

## 3. Screen (shared)

Post-action UI shows, for the player roster:

- Outcome (win / lose).
- Per ganger: alive/dead, carried injuries, **training** bumps (with special names), **XP** breakdown (participation / kills / survival / MVP) + bank.
- Continue → roster write-back → next battle or menu (mission chaining is a separate ticket).

Placeholder outcome + Continue stays until this data exists.

---

## 4. Open questions for sign-off

**Injury (678):** Accepted.

**Split (679):** Training = attributes; XP = skills only. MVP + survival = XP. Damage/morale survival = training.

Remaining:

1. **Downed:** Recover with ledger only for v0 (no capture)?
2. **XP mix:** Participation 2 + kill 3 + survival 1 + MVP 5 OK?
3. **Combined training table** (use + stress + feats + survive-damage) + hard max +3 OK?
4. **Skills:** bank visible, no purchase UI in v0 OK?

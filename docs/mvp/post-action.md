# Post-action — injury carry and advancement (MVP)

Design for the fight → consequences → next fight loop. Parent: post-action epic under Battlescape.

**Status:**

- **GTW-678 injury carry — Accepted** (in-battle injury table already defined; no second post-action roll).
- **GTW-679 advancement — proposed** (combined stress + feat training). Do not implement until Accepted.

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

1. **Use-based attributes** — train by using, stressing/failing, landing feats, and surviving bad fights (Xenonauts-style).
2. **XP** — a separate bank that **buys skills**, never raw attribute points.

### Accrual events (per player ganger, once at outcome latch)

| Event | When it counts | Default XP (tunable) |
| --- | --- | --- |
| **Participation** | Deployed and took at least one turn (or one spent TU) this battle | 2 |
| **Kill credit** | Attributed kill in the who-killed-whom log | +3 per kill |
| **Survival** | Alive at outcome (standing or Downed-recovered) | +1 |

No XP for taking damage or for enemy deaths you did not cause. Magnitudes live in tuning data (e.g. `advancement.tuning.ron`); tests assert **properties**, not hard-coded totals.

Enemy gangers: no XP track for MVP.

### Use-based attributes — combined layers

At outcome latch, for each living player ganger, sum every grant that fired, **per attribute**, then clamp.

**Hard ceiling: +3 per attribute per battle.**

```
bump = min(3, default_use + stress + feats + survive + mvp)
```

Each named grant fires **at most once per ganger per battle** (even if the act happened many times). Dead gangers get nothing.

Kill **XP** still stacks per kill. Attribute grants below are once-per-row.

---

#### Layer A — Default use (+1)

Ordinary fight work. Signal met → **+1**. Harder use does not stack more default.

| Signal | Attribute |
| --- | --- |
| ≥1 ranged attack resolved | Aim |
| ≥1 melee attack resolved | Strength |
| Cells entered / move acts ≥ N (tunable, default 5) | Speed |
| Took any HP damage and still conscious at some point after | Grit |

---

#### Layer B — Stress / failure (+1)

Train when the stat was **tested or failed**.

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Missed shot** | ≥1 ranged attack that did not connect (same miss the combat log records) | +1 | Aim |
| **Missed melee** | ≥1 melee attack that failed to connect | +1 | Strength |
| **Got hurt** | Took ≥1 non-graze wound (any severity) | +1 | Toughness |
| **Morale stress** | Lost any Morale this battle (when Morale is live; until then: suppression / Bottle hit if those signals exist, else defer) | +1 | Cool |
| **Overloaded** | Spent any turn over carrying capacity / Strength shortfall (when encumbrance exists; else defer) | +1 | Strength |
| **Pushed empty** | Ended ≥1 of own turns at 0 TU after spending TU that turn | +1 | Grit |

---

#### Layer C — Feats / critical success (+1, unless noted)

Clean wins and named battle feats. Combined from both earlier lists.

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

#### Layer D — Survive a bad wound (+1 / +2)

| Special | When | Extra | Attribute |
| --- | --- | --- | --- |
| **Walked it off** | Alive at outcome after taking a **Major** or **Critical** injury this battle | +1 | Grit |
| **Back from the brink** | Was **Downed** at least once and still alive at outcome | +2 | Grit |

These stack with default Grit / stress, then hit the +3 cap (e.g. default + brink alone already **Grit +3**).

---

#### Layer E — MVP (+1)

**One player ganger per battle** may earn MVP (on top of everything above).

| Rule | Detail |
| --- | --- |
| Who | Highest attributed kill count among living player gangers at outcome. Tie → most non-graze wounds inflicted. Still tied → lowest stable id (deterministic). |
| Reward | **+1** to the attribute that received the largest pre-MVP bump this battle; if none, **Cool**. |
| None | If no player ganger got a kill and no wounds inflicted, **no MVP**. |

---

### Worked examples

| Fight | What fires | Capped result |
| --- | --- | --- |
| Shot a lot, missed, no kills | Aim use +1, Missed shot +1 | **Aim +2** |
| Shot, missed, one ranged kill | use + miss + ranged kill | **Aim +3** |
| Two kills, multi-kill | Aim/Strength kill rows + Cool multi-kill +1 | as mapped |
| Major injury, Downed, recovered | Got hurt Toughness +1; Walked it off +1 Grit; Brink +2 Grit; use Grit if HP lost | **Toughness +1**, **Grit +3** (cap) |
| Solo survivor win | Last of the squad Cool +2 (+ other Cool if any) | **Cool** up to +3 |
| Squad ace | kills + feats + MVP +1 | best attr up to **+3** |

---

### Explicitly not training (v0)

Reload, stance change, open door, enter emplacement, empty grenade throw, panicking enemies by presence alone, raw damage totals without miss/hit/wound/kill events.

---

### XP bank vs skills

- Accrued XP sums onto the ganger’s **skill XP bank**.
- **Skill purchase UI and skill content are not MVP** unless a separate ticket lands them. Post-action shows the bank number so the track is visible.
- XP never converts into free attribute points.

### When

All of the above runs **once** when the battle outcome latches (same moment as BattleResults), before or as the post-action screen opens. Not per act.

### Build order after Accept

1. UsageTally + miss/hit/wound/kill/Downed/shove signals from act messages (shared log).
2. Advancement apply (pure sim, seeded tests) — all layers + +3 clamp + deterministic MVP.
3. Wire into BattleResults / roster fold.
4. Post-action UI: bumps + which specials fired (short names) + MVP badge.

Dependencies that gate individual rows: Morale stress needs Morale live; Overloaded needs encumbrance. Ship the rest first; those two rows stay defined but inactive until their systems exist.

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
3. **Combined training table** (use + stress + feats + survive + MVP) + hard max +3 OK?
4. **Skills:** XP bank visible, no purchase UI in v0 OK?

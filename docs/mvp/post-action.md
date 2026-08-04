# Post-action — injury carry and advancement (MVP)

Design for the fight → consequences → next fight loop. Parent: post-action epic under Battlescape.

**Status:**

- **GTW-678 injury carry — Accepted** (in-battle injury table already defined; no second post-action roll).
- **GTW-679 advancement — proposed** (use-bump cap + special list below). Do not implement until Accepted.

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

#### Special extras (closed list for MVP)

Specials are **extra** points on top of default use. Each special fires **at most once per ganger per battle** (even if the act happened many times). Values are **+1 or +2** only. Content/skills may add more rows later with the same rules.

| Special | When it fires (player ganger, living at outcome) | Extra | Attribute |
| --- | --- | --- | --- |
| **Ranged kill** | ≥1 attributed kill with a ranged weapon | +1 | Aim |
| **Melee kill** | ≥1 attributed kill with a melee weapon / unarmed strike | +1 | Strength |
| **Multi-kill** | ≥2 attributed kills (any weapon) | +1 | Cool |
| **Critical wounder** | Inflicted ≥1 Critical or Fatal named wound on an enemy (from the injury/wound log), whether or not they died | +1 | Aim if the hit was ranged, Strength if melee; if both kinds, one +1 to each (still two specials) |
| **Execute** | Successfully executed a Downed enemy | +1 | Cool |
| **Stabilize** | Successfully stabilized a Downed ally | +1 | Cool |
| **Shove finish** | Shoved an enemy who then died from the fall / shove outcome this battle | +1 | Strength |
| **Back from the brink** | Was Downed at least once this battle and is still alive at outcome | +2 | Grit |
| **Last of the squad** | Only living player ganger at outcome (others dead or never deployed) and battle was won | +2 | Cool |

Notes:

- **Kill specials ≠ XP kills.** Kill XP still accrues per kill; attribute specials fire once per row when the threshold is met.
- **Default use still applies** when you only shot and never killed (Aim +1 from use, no Ranged kill special).
- **No special for:** reloading, changing stance, opening doors, entering emplacements, throwing without a kill, panicking enemies, or raw damage totals. Those are ordinary use or not training.
- **Skills / mission rewards** later: same shape (named special, +1 or +2, one attribute). They count toward the cap.

#### Cap

**Hard ceiling: +3 to any one attribute from all sources in one battle.**  
Sum(default use + specials for that attribute), then `min(sum, 3)`.

Examples:

- Shot a lot, one ranged kill → Aim default +1 + Ranged kill +1 = **Aim +2**
- Shot a lot, multi-kill, critical ranged wound → Aim +1 default +1 kill +1 crit = **Aim +3** (cap)
- Downed and recovered, also took HP while up → Grit default +1 + brink +2 = **Grit +3**
- Only multi-kill Cool specials without Cool default use → Cool +1 (or +2 if last-of-squad), no default Cool yet

Rules:

- Only **living** gangers (Dead get no bumps).
- Bumps write to **base attributes** on the campaign roster, then re-derive combat stats for the next fight.
- No mid-battle permanent attribute growth.

Attributes without a default-use row in v0 (Toughness, Reflexes, Cool, Luck) can still gain from specials only (Cool is specials-heavy on purpose). Same +3 cap.

### XP bank vs skills

- Accrued XP sums onto the ganger’s **skill XP bank**.
- **Skill purchase UI and skill content are not MVP** unless a separate ticket lands them. Post-action shows the bank number so the track is visible.
- XP never converts into free attribute points.

### When

All of the above runs **once** when the battle outcome latches (same moment as BattleResults), before or as the post-action screen opens. Not per act.

### Build order after Accept

1. UsageTally from act messages.
2. Kill / wound attribution log (feeds XP + specials).
3. Advancement apply function (pure sim, seeded tests) — default use + special table + +3 clamp.
4. Wire into BattleResults / roster fold.
5. Post-action UI readout (list which specials fired).

---

## 3. Screen (shared)

Post-action UI shows, for the player roster:

- Outcome (win / lose).
- Per ganger: alive/dead, carried injuries (short text), XP gained this battle, use bumps this battle (with special names that fired), running skill XP.
- Continue → roster write-back → next battle or menu (mission chaining is a separate ticket).

Placeholder outcome + Continue stays until this data exists.

---

## 4. Open questions for sign-off

**Injury (678):** Accepted as written — carry ledger only; in-battle roll is the definition.

Remaining (679 / shared):

1. **Downed:** Recover with ledger only for MVP (no capture)?
2. **XP mix:** Participation + kill + survival OK?
3. **Default XP magnitudes:** 2 / 3 / 1 OK as tuning starting points?
4. **Default use map:** Aim / Strength / Speed / Grit OK?
5. **Special table + cap:** the closed list above (+1/+2 extras, hard max +3) OK? Any row to drop/add?
6. **Skills:** XP bank visible but no purchase UI in v0 OK?

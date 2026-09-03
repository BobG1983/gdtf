# Morale, Bottle, and nerve effects

Psychological combat track.

**Status: Accepted** (2026-08-04). Implementation may proceed against this doc; split build tickets from §8.

Inspired by UFO / Xenonauts *feel* (named break states, fire and casualties rattle people, suppression ≠ panic). **Not** a copy of their tables, psi, or officer math.

Related:

- Pools and derivation: [stats.md](stats.md) (Morale / Bottle already derived from Grit + Cool; not consumed yet).
- Physical mirror: [wounds-and-roster.md](wounds-and-roster.md).
- Suppression (live, separate): fire-radius pin in the sim — the rule it enforces is [§1.1 Breaking away from suppression](#11-breaking-away-from-suppression).
- Lasting campaign nerve scars: later, not this battlescape slice.
- Training hooks when live: [post-action.md](../mvp/post-action.md) (morale stress / held their nerve).

---

## 1. Two layers — do not merge them

| Layer | Job | Lifetime |
| --- | --- | --- |
| **Suppression** | Tactical pin: heads down, reaction fire gated, stance/stability hit | Clears on the ganger’s turn start (as today) |
| **Morale / nerve** | Psychological break: named **nerve effects**, then **Bottled** | Effect duration (below); Bottled until battle end |

Suppression may **deal a small Morale tick** but never *is* a nerve effect. A Cool ganger can be suppressed all fight and never panic.

### 1.1 Breaking away from suppression

A suppressed ganger is pinned where it stands. It may walk only if the walk **moves away** and then **gets out of the line of fire**:

1. **It moves away.** The destination is farther from the suppressor's cell than the start is, measured across x and y (Chebyshev — the larger of |dx| and |dy|).
2. **It gets out of the line of fire**, by either of two routes:
   - **it ends behind cover** — the cell one step from the destination toward the suppressor carries cover, **on the destination's own storey**; **or**
   - **it ends out of sight of the shot cell** — from the destination, the mover cannot see the cell the fire came from.

Moving away on its own is not enough, and neither route helps a walk that stays as close as it started. A walk that fails the distance test, or passes it and takes neither route out, is refused as suppressed before the TU pool is read at all, so a full pool does not buy a way out.

**The mover is the observer** in the sight test, and the cell the fire came from is the target. The ray is flown from the destination, at the mover's own stance and facing, to that cell — the same line-of-sight probe the shot pipeline uses ([visibility.md](visibility.md)). Sight is directional, so a ganger can break away while whoever fired can still see it. That is intended.

The shot cell records no stance, so the ray aims at it **standing**. That is the strictest choice: a lower aim point ducks behind cover the standing one clears, which would free walks this rule refuses.

The question asked is what the mover would see **standing on the destination**, so the mover's own body — still on the cell it is leaving until the walk runs — never counts as what hides it. Anyone else's does: a living ganger between the destination and the shot cell breaks the line like any other obstruction, and a body on the floor there — downed or dead — breaks it only at the LOW band ([resolution.md](resolution.md) §2), so a ray that stays above the floor crosses it, because this is the one geometry truth.

**Height is not distance.** The distance check ignores the storey. Climbing a floor gains no ground on the suppressor, so a suppressed ganger who moves straight up is refused however good the cover is up there, and however completely the climb breaks the line back to the shot cell. This is the rule, not a gap in it.

**Cover is read on the storey you end on**, never the shooter's. A shooter a storey above or below does not change which cell the check looks at: it stays the cell beside your destination, on your storey.

The same choice governs the **auto-stance drop**. When a ganger is first suppressed, cover in that cell — toward the suppressor, on the ganger's own storey — drops it to Prone behind Low cover and Crouching behind Mid or High cover.

---

## 2. Pools (mirror HP / Wounds)

| Pool | Role | Parallel |
| --- | --- | --- |
| **Morale** (current + max) | Stress buffer — psych HP | HP |
| **Bottle** (current + max) | How many hard breaks left — psych Wounds | Wounds |

**Derivation (already designed):**

- Max Morale = weighted sum of Grit + Cool (`GangerStatTuning.morale`).
- Max Bottle = round(Morale / `bottle_per_morale`) (≈10).

**Battle start:** current = max for both.

**Re-derive:** update max; clamp current ≤ max (same as other pools). Floor of max after injury Modify stays ≥ 1 where that rule applies.

**Cool** mitigates incoming Morale damage (tuning). **Grit** can speed mid-battle recover (optional leaf).

---

## 3. Shock pipeline

```
event (suppress / wound / ally falls / …)
  → Morale damage (Cool-mitigated)
  → maybe spend Bottle on a hard shock
  → if Bottle == 0 → Bottled (terminal for this battle)
  → else if shock qualifies → roll Nerve effect from table
```

### What counts as a shock

| Trigger | Typical Morale hit | Nerve roll? |
| --- | --- | --- |
| First suppressed this turn | Small | No (pin only) — unless Morale crosses a band |
| Non-graze wound on self | Medium | Yes if severity ≥ Major, or Morale ≤ 50% after hit |
| Self Downed | Large + Bottle spend | Yes (if still alive) |
| Ally Downed in LOS | Medium | Yes if Cool check fails / random band |
| Ally Dead in LOS | Large | Yes |
| Alone (no conscious ally within N cells) | Small tick per own turn | Only if Morale already low |
| Leader dead (if a leader tag exists later) | Faction-wide large | Yes |

Exact magnitudes live in `morale.tuning.ron` (or a leaf on combat tuning). Tests pin **properties**, not raw numbers.

### Recover (mid-battle)

Small Morale **heal** (not Bottle) when:

- Attributed kill
- Successful stabilize on an ally
- End of own turn in cover while not suppressed (tiny tick)

Recover never clears **Bottled**. Recover can clear soft nerve effects when Morale rises past a threshold (see §5).

---

## 4. Bottled (terminal psych state)

**Bottled** = out of this fight on the mind track (parallel to Downed/Dead on the body track).

| Rule | Detail |
| --- | --- |
| Enter | Bottle current hits 0 |
| Acts | None — no move, shoot, reaction, contextual |
| Targeting | Still targetable (enemy can finish / ignore); no squad FOV contribution |
| UI | Distinct from Downed (text + tint) |
| Battle end | Alive for roster (not Dead); carries any nerve scars later; counts as survival for XP if design says so |
| Player Flee button | Stays **voluntary gang abort** of the mission — not the same as per-ganger Bottled |

**Gang-wide bottle** (whole side routs): **not v0**. Per-ganger only.

**LifeState:** prefer extending with `Bottled` (or a dedicated marker if Alive must stay for hit rules). `is_active` for FOV/acts = Alive only.

---

## 5. Nerve effects (the “panic types” — our names)

Data-driven, same shape as injuries:

- Defs: `assets/content/nerve/<key>.nerve.ron`
- Weighting: by **severity band** (Light / Hard) and **trigger kind** (UnderFire / SelfWound / AllyDown / AllyDead / Isolated)

### Accepted rules

| # | Decision | Rule |
| --- | --- | --- |
| 1 | Stacking | **One active nerve effect** per ganger. New roll replaces only if new severity ≥ old; else keep current. |
| 2 | Duration | Soft effects: **until Morale recovers above 50% of max**, or **2 of the ganger’s turns**, whichever first. Hard effects: **rest of battle** unless replaced. Bottled: battle end. |
| 3 | Reckless | Forced **aggressive options only** (must advance or fire if able) — **no** friendly-fire fumble in v0. |
| 4 | Gang bottle | **Not v0.** |
| 5 | Recover | **Yes** — small Morale heal from kill / stabilize / quiet cover turn (§3). |

### v0 effect list

| Key | Severity | Behavior |
| --- | --- | --- |
| **hesitant** | Soft | +TU cost on acts (tunable); cannot enter Aim mode; no reaction fire |
| **shaken_aim** | Soft | Temporary Shooting (and Fight if melee) dock via modifier ledger for duration |
| **hunkered** | Hard | Forced prone or kneel; cannot leave current cover cell except to drop level; no advance |
| **jumpy** | Soft | On reaction opportunity: 50% skip (waste), else fire with wider cone (when reactions exist; until then: first shot each turn wider cone) |
| **reckless** | Hard | While active: legal acts are only move toward nearest visible enemy, melee, or fire; cannot Flee UI for this ganger; cannot stabilize |
| **dropped_focus** | Soft | Clear Aiming; exit emplacement; cancel aim toggle |
| **bottled** | Terminal | See §4 — not a replaceable soft effect |

**Not in v0:** psi / mind control, random team berserk, officer aura radius (can add later as Cool aura).

### Roll procedure

1. Shock applies Morale (and maybe Bottle) damage.
2. If Bottled → stop.
3. If trigger says “nerve roll” and ganger is active: sample weighting for (severity, trigger).
4. Apply one effect per §5 stacking rules.
5. Emit messages for act log + FCT.

Cool can shift severity band down one step (tunable), not delete the roll.

---

## 6. Relation to existing systems

| System | Interaction |
| --- | --- |
| **Suppression** | Unchanged pin; optional Morale damage on apply |
| **Injury ledger** | Separate; both can apply modifier docks |
| **AI** | Skip Bottled; if Reckless, prefer advance/fire; if Hunkered, stay in cover |
| **Act log** | `MoraleDamaged`, `NerveApplied`, `Bottled` deeds |
| **FCT** | Families for nerve names (like Suppressed tint) |
| **Stat block** | Morale / Bottle pips |
| **Post-action training** | Lost Morale → Cool train; held through loss without Bottled → “held their nerve” |

---

## 7. What we are not doing (v0)

- Copying UFO panic % tables or Xenonauts bravery formulas wholesale
- Psi / mind control
- Gang rout as automatic loss condition
- Lasting nerve injuries across battles (later)
- Replacing suppression with panic

---

## 8. Implementation order

1. Morale/Bottle current+max, damage API, re-derive clamp
2. `LifeState::Bottled` (or marker) + act gates
3. Suppression → Morale tick
4. Nerve registry + hesitant / shaken_aim / hunkered / bottled
5. Ally casualty shocks in LOS
6. Reckless, jumpy, dropped_focus
7. Recover ticks + UI pips + FCT/act log
8. Authoring guide `docs/authoring/nerve-authoring.md`

All core logic in `gdtf_battle_sim`, headless tests, seeded RNG.

File single-responsibility build tickets from this list under the battlescape and morale parents.

---

## 9. Acceptance record

**Accepted 2026-08-04** as written, including §5 defaults:

1. Suppression stays separate; may deal Morale damage
2. Morale + Bottle as psych HP / Wounds
3. One nerve effect at a time; soft vs hard duration as table
4. v0 effect list as table
5. Reckless without friendly fire
6. No gang-wide bottle in v0
7. Mid-battle Morale recover from kill / stabilize / quiet cover
8. Player Flee button remains voluntary mission abort

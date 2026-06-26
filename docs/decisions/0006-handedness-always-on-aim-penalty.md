---
name: "ADR 0006: Hand-disabling injury carries an always-on aim penalty"
description: The 1H aim penalty from a hand-disabling injury is a standing Modify(Shooting) through the modifier layer, not a firing-time cone term.
---

# 0006. Hand-disabling injury carries an always-on aim penalty

## Status

`Accepted` — 2026-06-26, GTW-443 (weapon `Handedness` + a hand-disabling arm injury).

## Context

GTW-443 adds weapon **handedness** (`OneHanded` / `TwoHanded`) and a hand-disabling
arm injury. Two consequences fall out of losing a hand:

1. **Firing eligibility** — a two-handed weapon needs both hands. This is a clean
   yes/no gate in the shared `can_fire` guard, derived on read from the injury ledger's
   hand count (see [resolution.md §6.1](../combat/resolution.md) and
   [stats.md](../combat/stats.md)). Not contentious.
2. **Aim degradation** — a ganger firing one-handed (a pistol it *can* still fire, or a
   weapon braced against a wrecked hand) shoots *worse*. The question this ADR settles is
   **where** that aim penalty lives.

The landed GTW-405/436 injury layer already gives every injury a `Modify(stat, amount)`
effect that the projector re-sums onto the derived stats each projection (the
`InflictedInjuries` ledger → `rederive_stats_on_injury_change` → cached `Shooting` →
`concentration_p` chain). The §1b cone reads `concentration_p(Shooting, weapon.accuracy)`.
So a `Modify(Shooting, −N)` already loosens the cone with zero new plumbing — *if* we
accept it as a standing stat shift. The alternative is a firing-time-only term: a penalty
applied at `fire()` when the shooter is down a hand, kept off the stat sheet.

## Decision

We will model the 1H aim penalty as an **always-on `Modify(Shooting, −N)`** effect carried
in the **same** hand-disabling injury's effects `Vec` (alongside its fieldless
`DisableHand`). It flows through the existing modifier layer onto the cached `Shooting`
component and from there into the §1b concentration exponent — **no** new firing-time cone
term, **no** `effective_shooting` accessor, **no** branch in `fire()`. The penalty applies
**whenever the injury persists**, not only while firing one-handed. The magnitude `N` is
authored **per injury** in its `.injury.ron` (a tunable), not a global constant.

The `DisableHand` effect itself is **inert** in the ledger's `gain` fold (it docks no stat
and accrues no bleed); the hand count is derived on read. The aim penalty is therefore the
*only* stat consequence, and it is a perfectly ordinary `Modify` — the new variant carries
the gate, the existing variant carries the number.

## Consequences

- **Zero new plumbing for the aim hit.** The penalty hot-reload-survives, composes with
  every other `Modify(Shooting)`, and shows on the inspect panel for free — it is just an
  injury effect like any other.
- **The penalty is standing, not conditional.** A one-handed-injured ganger shoots worse
  even with a one-handed weapon it can fully use. This is the approved default: a ruined
  hand is a ruined hand. If a future design wants the penalty to apply *only* when actually
  firing two-handed-worth of weapon one-handed, that is a new ADR — it would have to
  reintroduce a firing-time term this ADR deliberately avoided.
- **Determinism / replay unaffected.** The penalty lands through the same re-summed
  projection as every other injury delta; no new RNG, no order dependence.
- **Tuning lives with content.** Each hand-disabling injury sets its own `N`, so the
  severity of the aim hit is authored per condition, not hard-wired.

## Alternatives considered

- **A firing-time-only aim term in `fire()`.** Apply `−N` to the cone only when the shooter
  is down a hand at fire time, keeping `Shooting` clean. Rejected: it duplicates the cone's
  Shooting input at a second site, needs a new `effective_shooting`-style accessor + a
  `fire()` branch, would not show on the sheet/inspect panel, and would diverge from the
  re-summed modifier layer that every other injury already uses — more code for a *narrower*
  rule the user did not ask for.
- **No aim penalty at all (gate only).** Let a hand injury forbid two-handed weapons but
  leave aim untouched. Rejected: losing a hand with no shooting consequence understates the
  injury and wastes the modifier layer that already exists for exactly this.
- **A global aim-penalty constant.** A single tuned `N` for all hand injuries. Rejected: it
  forecloses authoring a worse hit for a worse injury; per-injury authoring is the house
  pattern for injury effects.

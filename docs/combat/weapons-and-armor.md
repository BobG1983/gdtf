# Weapons & Armor — damage resolution

How a hit resolves: weapon stats vs armor stats, the per-hit formula, and how the 7-type matchup wheel plugs in. The output feeds the HP / Wound model in [stats.md](stats.md) and [wounds-and-roster.md](wounds-and-roster.md).

> The *structure* is settled. All magnitudes (weapon and armor numbers, the matchup swing) are **TBD (tuning)**.

## The 7 types

One shared wheel — armor types **Plated, Refractive, Flak, Void, Hazard, Reinforced, Ceramic**, each the mirror of a weapon damage flavor (Shock, Blast, Chem, Kinetic, Plasma, Rend, Las respectively) at the same wheel node. Definitions, the wheel, and the underlying math: [matchup.md](matchup.md) and [two-paradox-tournament.md](two-paradox-tournament.md). Same-type (a weapon vs its own armor) is the **neutral** mirror; favorable / resisted come from the Paley wheel between *different* types.

## Weapon stats

- **damage** — base damage of a hit.
- **punch** — armor protection it ignores (penetration).
- **shred** — extra **integrity** damage per hit, on top of the normal soak/penetration wear. Shred attacks armor **durability** (breaks it sooner), not hardness.

## Armor stats

- **floor** — a hit that lands on armor always deals at least this (a vest still bruises).
- **protection** — damage reduction, down to `floor`.
- **integrity** — how much it can block before it's useless (durability).
- **hardness** — punch it ignores.

## Per-hit resolution

Weapon (`damage`, `punch`, `shred`) vs armor (`floor`, `protection`, `integrity`, `hardness`):

1. **Effective penetration:** `effPen = max(0, punch − hardness)`
2. **Damage to the ganger:** `dmg = max(floor, damage − max(0, protection − effPen))`
   — protection soaks, but penetration eats into protection, and the result never drops below `floor`.
3. **Armor integrity wears:** `integrity −= min(protection, damage) + effPen + shred` — the soak/penetration wear **plus the weapon's shred** (extra integrity damage). At `integrity ≤ 0` the armor is **useless** (stops protecting). **Hardness does not degrade** — shred attacks durability, not hardness.

Mid-battle wear is **live**: each hit's integrity result persists on a battle-local worn copy of the piece (a per-ganger `armor_at` field on the sim's ganger-state component/struct, keyed by body location), so armor worn to ≤ 0 stops protecting for the rest of the battle — later hits on that location resolve as bare flesh. The roster sheet itself never wears mid-battle; whether wear carries between missions is the open maintenance question below. (This worn copy lives in the render-free sim — `gdtf_battle_sim` — never in the presenter.)

`dmg` then feeds the HP / Wound model: it reduces **HP**, and **every hit** rolls **wound severity** (the sim's `roll_severity`) — a penetration-gated bucket roll: penetrating damage drives the score, **Toughness** mitigates, the struck part and the weapon's **Fatal-bias** push it up, both gangers' **Luck** bend the one-sided random tail; bucketed Minor→Fatal, and a low roll is a graze (no Wound). See [stats.md](stats.md) and [wounds-and-roster.md](wounds-and-roster.md).

## How the matchup wheel plugs in

The matchup is a **multiplier on punch and shred only** — nothing else. Everything downstream (damage, HP loss, wound-chance, severity, armor wear) ripples from that single hook.

- **Favorable** (weapon type beats armor type): punch & shred **×1.33**.
- **Resisted** (weapon type loses): punch & shred **×0.34**.
- **Neutral** (same type / mirror): **×1**.

The swing is **asymmetric** — the resisted penalty (−66%) is double the favorable bonus (+33%), so players avoid bad matchups, not just chase good ones. A multiplier is scale-independent and always proportionally felt without ever bypassing armor. The **±33%/66%** values are **tuning defaults** (carried in a tuning-config Bevy `Resource`, not hardcoded).

## TBD (tuning / design)

- All numbers (weapon damage/punch/shred; armor floor/protection/integrity/hardness; the matchup swing).
- Weapon archetypes / profiles (autogun, lasgun, **plasma cannon** = high damage + high punch + **Fatal-bias**, chainsword = high shred, …).
- **Fatal-bias:** how specific weapons push the severity roll toward **Fatal** (a plasma cannon to an unarmored chest).
- Whether `integrity` / `hardness` repair between missions (armor maintenance) or degrade for good — cross-battle carry of mid-battle wear is open.

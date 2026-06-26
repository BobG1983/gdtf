# Weapons & Armor — damage resolution

How a hit resolves: weapon stats vs armor stats, the per-hit formula, and how the 7-type matchup wheel plugs in. The output feeds the HP / Wound model in [stats.md](stats.md) and [wounds-and-roster.md](wounds-and-roster.md).

> The *structure* is settled. All magnitudes (weapon and armor numbers, the matchup swing) are **TBD (tuning)**.

> **Where equipment lives (the ECS shape).** The stat components below live on the related **weapon** and **armor-piece entities**, not on the ganger: a ganger `Wields` a weapon entity (carrying the weapon stats) and `Wears` six armor-piece entities (each carrying its piece stats), via Bevy ECS relationships. This page defines the stat *vocabulary*; for *where* those stats are stored — and why — see [ADR 0004](../decisions/0004-equipment-as-entities-relationships.md).

## The 7 types

One shared wheel — armor types **Plated, Refractive, Flak, Void, Hazard, Reinforced, Ceramic**, each the mirror of a weapon damage flavor (Shock, Blast, Chem, Kinetic, Plasma, Rend, Las respectively) at the same wheel node. Definitions, the wheel, and the underlying math: [matchup.md](matchup.md) and [two-paradox-tournament.md](two-paradox-tournament.md). Same-type (a weapon vs its own armor) is the **neutral** mirror; favorable / resisted come from the Paley wheel between *different* types.

## Weapon stats

- **damage** — base damage of a hit.
- **punch** — armor protection it ignores (penetration).
- **shred** — extra **integrity** damage per hit, on top of the normal soak/penetration wear. Shred attacks armor **durability** (breaks it sooner), not hardness.
- **handedness** — `OneHanded` or `TwoHanded` (GTW-443). A one-handed weapon (a pistol) can be fired with a single working hand; a two-handed weapon (a long-arm or heavy piece) needs **both** hands. The shared firing guard (`can_fire`) refuses a two-handed weapon once a hand-disabling injury leaves the shooter with fewer than two hands — see the **Hand count** note in [stats.md](stats.md) and the always-on 1H aim penalty in [resolution.md](resolution.md). Handedness gates **fire only** — it never touches the `Wields` relationship (a one-armed ganger keeps the long-arm slung, just can't fire it).

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

Mid-battle wear is **live**: each hit's integrity result persists on the battle-local **armor-piece entity** the ganger `Wears` for that body location (its `integrity` component, keyed by `BodyPart`; see [ADR 0004](../decisions/0004-equipment-as-entities-relationships.md)), so armor worn to ≤ 0 stops protecting for the rest of the battle — later hits on that location resolve as bare flesh. The roster sheet itself never wears mid-battle; whether wear carries between missions is the open maintenance question below. (These battle-local piece entities live in the render-free sim — `gdtf_battle_sim` — never in the presenter.)

`dmg` then feeds the HP / Wound model: it reduces **HP**, and **every hit** rolls **wound severity** (the sim's `roll_severity`) — a penetration-gated bucket roll: penetrating damage drives the score, **Toughness** mitigates, the struck part and the weapon's **Fatal-bias** push it up, both gangers' **Luck** bend the one-sided random tail; bucketed Minor→Fatal, and a low roll is a graze (no Wound). See [stats.md](stats.md) and [wounds-and-roster.md](wounds-and-roster.md).

## How the matchup wheel plugs in

The matchup is a **multiplier on punch and shred only** — nothing else. Everything downstream (damage, HP loss, wound-chance, severity, armor wear) ripples from that single hook.

- **Favorable** (weapon type beats armor type): punch & shred **×1.33**.
- **Resisted** (weapon type loses): punch & shred **×0.34**.
- **Neutral** (same type / mirror): **×1**.

The swing is **asymmetric** — the resisted penalty (−66%) is double the favorable bonus (+33%), so players avoid bad matchups, not just chase good ones. A multiplier is scale-independent and always proportionally felt without ever bypassing armor. The **±33%/66%** values are **tuning defaults** (carried in a tuning-config Bevy `Resource`, not hardcoded).

## Weapons — identity, fire modes, and authoring

A weapon is **not** one packed struct — it is an ECS **component bundle** (a `Weapon` marker plus a `WeaponName` and one stat component per number) spawned onto the armed ganger entity (GTW-200). Every weapon is **authored data**: a loose per-file `assets/weapons/<key>.weapon.ron` deserialised into a `WeaponSpec`. At battle setup the whole `assets/weapons/` folder loads into a name-keyed `WeaponRegistry`; each `GangerSpawn` references a weapon **by key**, and `setup_battle` resolves the key → inserts the `WeaponBundle` onto the ganger (mirroring how `WornArmor` is seeded) — a missing key is a handled error, never a panic (GTW-257). The dedicated `.weapon.ron` extension keeps the folder load unambiguous among GDTF's other `.ron` asset types. Nothing about a weapon is hardcoded.

### Weapon name

Every weapon carries a **`WeaponName`** — its human-facing identity (e.g. "autogun", "lasgun"). It is **not** authored as a field; it is the `.weapon.ron` filename **stem** (so `autogun.weapon.ron` → `WeaponName("autogun")`), which is also the registry key a ganger references. The name is **not** read by the combat-math layer (§1/§6) — it is queried only for UI (the status panel and the fire-mode picker, GTW-254/256).

### Fire-mode selector — an authored list, not a fixed ladder

A weapon's **fire-mode selector** is a **`FireMode(Vec<FireModeSpec>)`** — the authored list of modes the weapon offers, **in authored order, Single first by convention**. It is **any subset of {Single, Burst, Full}**: this broadens the earlier "three fixed ladders" (single / single+burst / single+burst+full-auto) so a weapon can offer, say, just single+burst. Each entry is a `FireModeSpec` carrying:

- **`kind: ModeKind`** — a closed enum `Single` / `Burst` / `Full`; its `Display` is the human label ("single" / "burst" / "full-auto") — there is **no** stored name string (GTW-260).
- **`cone_mult: ModeConeMult`** — the mode's selector cone multiplier (the §1 `firemode` term: single ≈ 1, full-auto ≥ 1).
- **`tu_percent: ModeTuPercent`** — the fraction of the shooter's TU pool a shot in this mode costs.
- **`shots: ModeShots`** — rounds fired per shot action.

The player picks the active mode through a **click-to-select popup picker** (GTW-254) — it lists exactly the weapon's offered modes and replaced the old blind cycle; the active mode shows on the action bar and rides in the input layer's `SelectedFireMode`. The code is **defensive**: a selector with no `Single` (or none at all) falls back to the first mode, then to a structural single-shot default — it never panics.

Illustrative authoring (the magnitudes are **tuning data** in the `.weapon.ron`, not pinned by tests):

```ron
// autogun.weapon.ron — offers all three modes
fire_mode: [
    (kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1),
    (kind: Burst,  cone_mult: 1.3, tu_percent: 0.45, shots: 3),
    (kind: Full,   cone_mult: 1.7, tu_percent: 0.60, shots: 8),
]
// lasgun.weapon.ron — offers only single + burst
fire_mode: [
    (kind: Single, cone_mult: 1.0,  tu_percent: 0.25, shots: 1),
    (kind: Burst,  cone_mult: 1.15, tu_percent: 0.40, shots: 3),
]
```

## TBD (tuning / design)

- All numbers (weapon damage/punch/shred; armor floor/protection/integrity/hardness; the matchup swing).
- Weapon archetypes / profiles (autogun, lasgun, **plasma cannon** = high damage + high punch + **Fatal-bias**, chainsword = high shred, …).
- **Fatal-bias:** how specific weapons push the severity roll toward **Fatal** (a plasma cannon to an unarmored chest).
- Whether `integrity` / `hardness` repair between missions (armor maintenance) or degrade for good — cross-battle carry of mid-battle wear is open.

# Wounds & Roster — the Heart of the Generator

A named persistent roster + permadeath + the Injury table is **~80% of the situation generator**. Everything memorable comes from here. The damage/stat model these hook into lives in [stats.md](stats.md).

## The Injury table is the highest-ROI thing in the design

It is almost pure **data and text** — infinitely generative, no art required, and it's where every story lives:

> *"Leader lost an eye → −1 to hit at long range → now has a vendetta against the gang that did it."*

One roll yields a mechanical consequence (a stat hit *felt* next fight) **and** a narrative hook **and** (later) a seed for the grudge system. One table, three payoffs.

## From damage to injury (the flow)

- In battle, damage depletes **HP**. **Every hit rolls for a Wound** — one severity bucket roll per hit (the sim's `roll_severity`), gated by **penetrating damage** (post-armor, so a weak hit can't reach the severe buckets) and mitigated by **Toughness**; some weapons bias toward severe/Fatal wounds (a chainsaw is hard to shrug off — a weapon's `fatal_bias`). A roll under the first bucket edge is a **graze**: HP loss only, no Wound. Full damage model: [stats.md](stats.md).
- A Wound **lands immediately** — location (the struck part) × severity (the bucket roll). The named-condition **Injury table is BUILT**: a non-graze, non-fatal wound draws one named injury from the weighted `(category, context, severity)` table — the wound's **source** (ranged / melee / fall) selects which per-source weighting of the ONE shared per-category pool is sampled — and freezes it onto the `HitReport`; all three sources — **ranged**, **melee** and **fall** — roll injuries in play today, each routing its blow through the ONE shared wound core; the `apply_injury` boundary then inflicts it — stat deltas land on the modifier layer, bleed accrues, hands or movement may be affected. The injury applies for the **rest of this battle** *and* is recorded as a campaign consequence — **even if the ganger finishes the fight standing**. A won fight can still cost you: every wound counts.
- Each injury **spends the ganger's Wounds budget by its severity** (below). **Wounds is the *life* pool — when it hits 0 the ganger is DEAD**, whether from stacked injuries or one **Fatal** hit, **even at full HP**.
- **HP and Wounds have different terminal states: HP ≤ 0 → Downed (alive); Wounds ≤ 0 → Dead.** HP damage never kills directly — it downs (see the state machine below).

### Injury asset and authoring surface (BUILT — /437/440)

Injuries are authored as `.ron` files loaded from `assets/content/injuries/<category>/` — one file per named condition. Four **category pools** match the four body-part groups:

| Category folder | Draws from |
|-----------------|-----------|
| `head/` | head injuries (Aim / Cool effects) |
| `torso/` | torso injuries (Toughness / HP effects) |
| `arm/` | **both** arms share this pool (Strength / Aim / DisableHand) |
| `leg/` | **both** legs share this pool (Speed / MovementCostMul) |

A **per-side** `BodyPart` (`LeftArm` / `RightArm` / `LeftLeg` / `RightLeg`) maps to its shared category via `BodyPart::injury_category()` — the exact struck side is still recorded (e.g. for `DisableHand`), but the injury is drawn from the shared pool.

**Weighting files** live at `assets/content/injuries/weighting/*.weighting.ron` — one per `(category, context)` (each file authors a `category:` and a `context:` — `Ranged` / `Melee` / `Fall`), authoring which named injuries appear in each severity bucket (`minor` / `major` / `critical`) and at what relative weight. The SAME shared per-category injury defs are weighted differently per source — e.g. a `broken_nose` is high in the melee table, near-zero in the ranged table — with no def duplicated. An unknown injury key in a weighting file `warn!`s at load time and is skipped. An injury registered but referenced by no weighting bucket in **any** context `warn!`s — it can never be rolled. **The missing-weighting `WARN` is a content authoring signal, never a crash.** **Which tables are sampled in play:** all three — the ranged tables (`<category>.weighting.ron`) by the ranged fire fold, the melee tables (`<category>.melee.weighting.ron`) by the melee strike (`melee::resolve_melee_strike`, which routes its blow through the shared wound core carrying `DamageContext::Melee`), and the fall tables (`<category>.fall.weighting.ron`) by the fall damage path. See the full authoring guide: [authoring/injury-authoring.md](../authoring/injury-authoring.md).

## Rolling an injury — location × severity

- A Wound's **body location is the struck part** — the hit's weighted location roll (head / torso / each arm / each leg; the sim's `roll_body_part` over the tuning-config `body_part_weights`) — and its **severity** is rolled per hit.
- **Severity** is a **bucket roll** (`roll_severity`): **penetrating damage** drives the score, **Toughness** mitigates, the **struck part** nudges it (head/torso wound worse than limbs), the weapon's **fatal-bias** pushes up the table, and **Luck is directional fortune** — the shooter's Luck adds to the score, the defender's Luck shrinks the one-sided random spread. The gate is penetration, not a Toughness threshold: a graze **can't** crit (no 1-damage amputations) — a low score is **no wound at all**.
- Location × severity resolves to a **named condition + stat hit** — e.g. head → Aim or Cool; leg → Speed; arm → Strength or Aim; torso → Toughness or HP. The condition carries its own rule AND docks the relevant **attribute**, which ripples through the computed stats (see [stats.md](stats.md)). **BUILT** — the injury table draws from RON assets at `assets/content/injuries/<category>/`; the weighting files at `assets/content/injuries/weighting/` control bucket membership and relative probability.

### Severity tiers — acute + residual + wound-budget cost

Every injury has an **acute** part (temporary, heals over downtime) and a **residual** part (what remains after healing), and costs **Wounds-budget** by tier:

| Tier | Acute (while healing) | Residual (permanent) | Wounds cost |
|------|----------------------|----------------------|-------------|
| **Minor** | usable — a flesh wound, light penalty that fades | none | 1 |
| **Major** | benched until healed | small (e.g. −1 to an attribute; broken bone) | 2 |
| **Critical** | benched; healing is **riskier** | large (e.g. lost eye, lost limb) | 3 |
| **Fatal** | — | **dead** — outright, skips Downed | — |

(The 1/2/3 cost split is a starting point — could be 1/2/4, 1/3/5, … **TBD (tuning)**.) Because severity spends the single Wounds (life) pool, a Critical both maims harder *and* pushes the ganger toward **death** faster.

**Fatal** is the top of the ladder — reached by extreme penetrating damage + weapon fatal-bias against low Toughness / armor (a plasma cannon to an unarmored chest). It kills on the spot, skipping Downed. **Outright death comes only from the injury table, never from HP loss.**

**Critical complication (optional / dial):** Criticals are where the infection / fails-to-heal **lethal risk** lives — the one path death takes past the battle. Off by default or behind a difficulty dial. **TBD.**

## Recovery

Recovery follows the severity tiers: **Minor** self-clears (the ganger stays available); **Major** and **Critical** require **downtime** — benched until the acute part heals, then carrying the permanent residual. Downtime length scales with tier (**TBD (tuning)**) and forces roster-depth decisions. The optional Critical complication is the only route to *post-battle* death.

**Post-action for MVP:** there is **no second injury-table roll** after the fight. Named injuries already landed in battle; aftermath **carries the ledger** onto the roster. See [post-action.md](../mvp/post-action.md) (proposed).

## Downed → death / capture / recover (the state machine)

Two pools, two outcomes: **HP ≤ 0 → Downed**; **Wounds ≤ 0 → Dead**. Both terminal gates are part of the authoritative sim's apply-hit path, with Dead trumping Downed when both trip in one hit.

**Downed** (HP gone, Wounds remaining) — alive, incapacitated, dying. The in-battle from-Downed flow (bleed-out + execute; stabilize) is part of the sim's per-turn tick and action handling; the battle-end arm (recover / capture) is campaign scope, not yet built. From there:

- **Bleed-out:** each turn down stacks a *Bleeding Out* condition draining a flat `bleed_rate` of Wounds — a clock you can read ([resolution.md](resolution.md) §9); Wounds emptying **kills**.
- **Stabilize:** an ally action (medic / first aid) at arm's reach halts the bleed-out — no new *Bleeding Out* stacks, Wounds already lost stay lost → **stable** (alive, out for the rest of the mission).
- **Execute:** an enemy can finish a Downed ganger → **dead**.
- **Battle ends** with the ganger still alive (downed or stable) → **recovered** if your side holds them / the field, or **captured** if they're in enemy turf or you lost the turf you were defending. Capture is a **turf outcome** that seeds **rescue missions** (full design: [campaign.md](../mvp/campaign.md), deferred past MVP).

**Dead** (Wounds ≤ 0) — Wounds is the **life pool**. It empties from **stacked injuries** over a fight or from a single **Fatal** injury (a plasma cannon to an unarmored chest), and it can hit 0 **even at full HP** — outright death that skips Downed. The injury table, never HP loss, is what kills.

## The psychological mirror (Morale & Bottle)

The psychological track mirrors the physical one (see [stats.md](stats.md)): **Morale** is psychological HP, **Bottle** is psychological Wounds. Losing the psychological fight produces named **nerve effects** and ultimately **Bottled** — out of the fight on the mind track. Suppression stays a separate tactical pin.

Full design: [morale.md](morale.md) (GTW-40, accepted). Lasting campaign nerve scars: later (GTW-407).

## Roster persistence

- Named gangers with stats, equipped gear (carrying a damage type — see [matchup.md](matchup.md)), accumulated injuries, and XP/skills.
- The roster is **carried forward** between missions. In the MVP this is just running 2–3 missions back-to-back; later it's owned by the campaign layer ([campaign.md](../mvp/campaign.md)).
- This is a database + consequence tables — data and rules, not engine grind. It lives in the **render-free sim crate** (`gdtf_battle_sim`) as plain typed Rust (structs + functions, no Bevy rendering deps), separate from the presenter / scene layer, **deterministic and seed-replayable** (injected seeded RNG) so we can *unit-test that the generator produces interesting outcomes*.

## Advancement

- **Training improves attributes**; **XP buys skills only** — never each other's job. See [stats.md](stats.md) and [post-action.md](../mvp/post-action.md) (proposed).
- Advancement and injury are the two opposing forces shaping a ganger over a campaign (gains vs scars).

## Grudges (logged now, surfaced later)

The grudge system — gangs/gangers that *remember* who crippled whom — is the design's moat (pillar 4), but it is **deferred** past the MVP. For v0, **just log who-killed-whom and who-wounded-whom** as plain data on the injury/death event. That log is the raw material the grudge system will later consume. Full design: [campaign.md](../mvp/campaign.md).

## TBD (design) checklist

- The Injury table entries per **location × severity** (highest-leverage content in the game).
- ~~The wound-chance formula~~ — **designed** (`roll_severity`): every hit rolls one bucket — severity = `j·pen_dmg + part_mod + fatal_bias − k·Toughness + I·Luck_shooter + roll(−L·Luck_defender .. R)` → tier via `severity_edges` (the defender's Luck extends the roll's floor downward — a shrug-off chance — while the ceiling stays `R`; below the first edge = no wound). Location-**dependent**: the struck part nudges the score (a per-body-part `severity_mod`). Remaining = tuning magnitudes (the tuning-config severity group, per-weapon Fatal-bias).
- The **Wounds-budget cost** per tier (1/2/3 vs 1/3/5, …) — tuning values.
- ~~The **down → death / capture / recover** state machine~~ — **designed** ([resolution.md](resolution.md) §9 + the state machine above): Downed → a stacking *Bleeding Out* condition drains Wounds; stabilize / execute / recover / capture. The two terminal gates plus the full in-battle arm (bleed-out + execute; stabilize) are part of the authoritative sim's hit/tick path; recover / capture at battle-end remain campaign scope.
- ~~The **psychological-effects / nerve** table~~ — **accepted** in [morale.md](morale.md); lasting campaign nerve still TBD (GTW-407).
- The Critical infection / **lethal-risk** system + difficulty dial.
- Downtime lengths per tier; capture & rescue rules (with the campaign layer).

# Two-Paradox Tournaments & the 7-Type Wheel — the math

Why the weapon/armor matchup ([matchup.md](matchup.md)) is built on a **2-paradox tournament** of exactly 7 types, what that structure guarantees, and how it maps to game systems. This is background; the gameplay rules (tuning, UI) live in [matchup.md](matchup.md).

## 1. Rock-paper-scissors is the minimal non-transitive cycle

RPS = 3 moves, each beats exactly one and loses to exactly one. Perfectly symmetric, balanced (equal win rates), and the *smallest* complete non-transitive relation. The trap with "improving" RPS: most ad-hoc additions break the symmetry and hand one move a dominant edge.

## 2. n-paradoxical tournaments

A **tournament** is a complete oriented graph — every pair of moves joined by exactly one directed "beats" edge. It is **n-paradoxical** if *every* set of `n` moves has a common predecessor: for any `n` enemy moves there is always a single move that beats all of them. (OEIS A362137.)

- **1-paradox** = ordinary RPS: smallest size **3**; for any 1 move there's a move that beats it.
- **2-paradox**: smallest size **7**; for any *two* moves there is always a single move that beats both. ← what we use.

Sizes grow brutally: **3, 7, 19, 67, 331, 1263** for n = 1…6 — all primes ≡ 3 (mod 4), constructed as directed **Paley graphs**. Nothing is known past n=6.

## 3. The 7-node construction (Paley / quadratic residues mod 7)

Types are `0..6`. The textbook orientation: type `x` **beats** `x+1`, `x+2`, `x+4` (mod 7) — those offsets are the quadratic residues mod 7. Each type beats 3 and loses to the other 3.

```rust
// The textbook residue orientation — NOT the shipped game table (see below).
// Beats these 3; loses to the other 3; neutral vs itself.
const BEATS: [[u8; 3]; 7] = [
    [1, 2, 4], // 0
    [2, 3, 5], // 1
    [3, 4, 6], // 2
    [4, 5, 0], // 3
    [5, 6, 1], // 4
    [6, 0, 2], // 5
    [0, 1, 3], // 6
];
```

The **shipped game table is the converse orientation** — weapon node `w` penetrates `w+3, w+5, w+6` (mod 7), the quadratic *non-residues* (see [matchup.md](matchup.md) "Drop-in data" and `crates/gdtf_battle_sim/src/damage_resolution/matchup/wheel.rs`). Identical λ=1 structure, every edge reversed; everything below holds for both orientations.

## 4. Properties we get for free

- **Regular tournament.** Every type beats exactly 3 and loses to exactly 3 → equal win rates, no dominant strategy. Balanced *by construction*, not by hand-tuning.
- **Doubly regular, λ = 1 (the key property).** For any *pair* of types there is **exactly one** type that beats both — not zero (always an answer), not several (the answer is specific).
- **Self-converse.** The structure is symmetric under reversing every edge, so the same guarantees hold on defense: any pair of incoming weapon types has exactly one armor that resists both.
- **7 is the floor.** You cannot drop to 5 or 6 and keep the pair-guarantee. 3 nodes is just RPS (the 1-paradox case).

## 5. How the math maps to game systems

- **One shared 7-type wheel** for both offense and defense (Pokémon-style single type system): each weapon emits a type, each armor **is** one type, one tournament lookup resolves a hit (the matchup lookup in `crates/gdtf_battle_sim/src/damage_resolution/matchup/wheel.rs`). Players learn 7 things + a pattern, not 49 cells.
- **Modifier, not auto-win.** Favorable / neutral / resisted multiplies weapon **punch & shred** only (numbers in [matchup.md](matchup.md)). The structure only pays off if matchups are *felt* in the decision — but it must never decide a fight alone.
- **Two threats = solvable; three threats = tradeoff.** By λ=1, two enemy armor types have exactly one weapon that answers both. Three armor types have **no** single weapon covering all three (three-way coverage would need a 3-paradox tournament, minimum **19** nodes) → the player must split loadout or accept a bad matchup. That escalation falls straight out of the math.
- **Spatial / cellular-automaton framing (geoscape, deferred).** Running the 7-type interactions over a grid produces shifting fronts and spirals — a candidate texture for the hex turf layer. **TBD (design)**; see [../mvp/campaign.md](../mvp/campaign.md).

## Sources

"Rock Paper Scissors 2" (Fractal Philosophy) + OEIS A362137 (n-paradoxical tournaments / directed Paley graphs).

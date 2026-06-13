# Pillar 7: Balanced by Construction

**The combat math is where the game lives or dies — structure it so no dominant strategy can emerge.**

A situation generator only generates *interesting* situations if the underlying math never collapses into a degenerate loop or a single optimal play. The moment one strategy strictly dominates, the generator stops generating — every situation resolves the same way.

So we prefer mechanisms that are **balanced by their structure** over balance we hand-author and constantly patch. The two-paradox matchup wheel is balanced by construction (every type beats exactly three and loses to three; any two threats have exactly one answer). Scale-independent multipliers (matchup, melee margin) stay correct no matter how the numbers move. Risk-based balancing (melee's power offset by the danger of closing) beats arbitrary damage caps.

Concretely: hyperfocus the tuning budget on the combat math, not the renderer; favor formulas whose fairness is provable; and treat a discovered dominant strategy as a structural bug, not a number to nudge. The math is the product.

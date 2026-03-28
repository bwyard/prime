# ADR-006: Causal Irrecoverability Lemma

**Status:** Accepted
**Date:** 2026-03-28

## Context
PRIME forbids mutation (ADR-001) and uses only four temporal operations (ADR-003). The deeper justification is a formal claim about information loss.

## Decision
**Mutable state is lossy compression of time.** When a value is overwritten, the previous state is irrecoverably lost. The write operation `x = f(x)` destroys the pre-image — given only the current value of `x`, you cannot recover what `x` was before the assignment.

This is the Causal Irrecoverability Lemma: mutation severs the causal chain. If a system mutates state at step N, no observer at step N+1 can distinguish the history that produced that state from any other history that arrives at the same value.

PRIME avoids this by returning new state as a tuple: `(result, next_state) = f(state)`. Every intermediate value is preserved in the call chain. The caller decides what to keep and what to discard — information loss is explicit, never implicit.

## Consequences
- **Positive:** Full causal history is recoverable. Any intermediate state can be inspected, logged, or replayed.
- **Positive:** Debugging is trivial. Every step is a pure function from previous state to next state.
- **Positive:** Provides the formal foundation for ADR-001 (no mutation) and ADR-003 (append-only state).
- **Negative:** Memory usage grows with history length if all intermediate states are retained.
- **Mitigation:** The caller controls retention. Folding discards intermediate states explicitly — the loss is a conscious choice, not a side effect of the programming model.

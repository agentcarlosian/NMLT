# Examples

Examples are design fixtures for proposed NMLT syntax. The current frontend
checks only top-level `system Name { ... }` structure and balanced delimiters.
It does not type-check or verify the declarations inside a system.

- `technicus/provider_attempt.nmlt`: durable external-effect lifecycle.
- `hyperbook/one_bit_clock.nmlt`: small temporal state machine.
- `agents/trust_chain.nmlt`: explicit authority propagation across agents.

Every example should eventually include intended claims, negative controls,
expected evidence class, and a concrete implementation or trace mapping.

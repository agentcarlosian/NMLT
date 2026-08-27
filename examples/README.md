# Examples

Examples are design fixtures for proposed NMLT syntax. The current frontend
checks only top-level `system Name { ... }` structure and balanced delimiters.
It does not type-check or verify the declarations inside a system.

The ten-file v1 corpus, intended claims, and negative controls are frozen in
[`CANONICAL.md`](CANONICAL.md) and [`canonical-v1.json`](canonical-v1.json).
Run `python3 tools/canonical_examples.py` from the repository root to verify
that all exact source identities still match.

Every example should eventually include intended claims, negative controls,
expected evidence class, and a concrete implementation or trace mapping.

Paper 1 (hidden ping / receive) lives outside the frozen v1 corpus, as a
design fixture only: [`paper1/hidden_ping_receive.nmlt`](paper1/hidden_ping_receive.nmlt)
(Lean small-model `Receiver`) and
[`paper1/receptive_receiver.nmlt`](paper1/receptive_receiver.nmlt)
(OpenSystem-receptive dual, not a replacement).
Do not add either to `canonical-v1.json` without a corpus-identity bump.

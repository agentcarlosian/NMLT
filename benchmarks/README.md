# Benchmarks

NMLT benchmarks measure semantic discrimination, evidence accuracy, and
counterexample quality—not parser acceptance alone.

The first suite models four independent provider-attempt defects. All semantic
expectations are marked `planned` until a semantic checker exists. The current
frontend should structurally accept these fixtures; doing so is not a benchmark
pass.

Each future executed benchmark run must bind:

- exact source and checker revisions;
- property and result class;
- bounds and assumptions;
- structured witness where refuted;
- negative-control identity;
- residual gaps.

# NMLT Build Week judge quickstart

## Fastest path: one command, no build

Supported prebuilt platform: Ubuntu 24.04 x86-64 with glibc 2.39.

Requirements after download:

- Python 3.11 or newer
- Bash and standard GNU core utilities
- No network access

Run:

    ./judge-demo.sh

The command exits zero only when all expected controls occur:

- The preserved workflow completes finite exploration and all four properties hold.
- The manually modeled dropped guard is refuted with the expected counterexample.
- A prior report is rejected after the exact NMLT model source bytes change.

Use this for paced screen output:

    ./judge-demo.sh --paced

## Assurance boundary

The C-to-Rust files provide a concrete review scenario. Their relevant behavior
is manually abstracted into NMLT. NMLT does not parse C or Rust, translate
between them, prove source equivalence, or establish native memory safety.

model_checked means the reported finite model was explored completely within
the displayed bounds. It is not an unbounded source-code proof.

## Build from source

Requirements:

- Rust 1.94.0 through rustup
- Python 3.11 or newer
- Bash and GNU Make

Commands:

    cargo build -p nmlt-cli --release
    ./judge-demo.sh --nmlt target/release/nmlt

The judge demo does not require Lean, TLC, Quint, P, Node, or network access.
Those tools belong to separate repository research and comparison gates.

## Pinned release

Release tag:

    build-week-judge-demo-2026

Repository:

    https://github.com/agentcarlosian/NMLT

Verify the downloaded archive with the adjacent .sha256 asset before
extracting it. The bundle also contains SHA256SUMS for its internal files.

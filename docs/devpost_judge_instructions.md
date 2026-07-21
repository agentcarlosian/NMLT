# Installation and testing: NMLT developer tool and CLI

## Fastest judge path: prebuilt, no build

Supported prebuilt platform: Ubuntu 24.04 x86-64 with glibc 2.39.

Requirements: Python 3.11 or newer, Bash, and standard GNU core utilities.
No network access is needed after the two release assets are downloaded.

    curl -LO https://github.com/agentcarlosian/NMLT/releases/download/build-week-judge-demo-2026/nmlt-build-week-judge-demo-2026-linux-x86_64.tar.gz
    curl -LO https://github.com/agentcarlosian/NMLT/releases/download/build-week-judge-demo-2026/nmlt-build-week-judge-demo-2026-linux-x86_64.tar.gz.sha256
    sha256sum -c nmlt-build-week-judge-demo-2026-linux-x86_64.tar.gz.sha256
    tar -xzf nmlt-build-week-judge-demo-2026-linux-x86_64.tar.gz
    cd nmlt-build-week-judge-demo-2026
    ./judge-demo.sh

The one command runs three live controls:

1. It completely explores the accepted finite workflow within the displayed bounds.
2. It refutes a manually modeled C-to-Rust dropped-guard scenario and prints the exact counterexample.
3. It changes the exact NMLT model bytes and rejects the previously saved result as stale.

The C and Rust snippets are scenario context. Their relevant behavior is
manually abstracted into NMLT. NMLT does not parse or translate C/Rust, prove
source equivalence, or prove native memory safety. model_checked is a complete
result for the displayed finite model and bounds, not an unbounded source-code
proof.

## Build from source

    git clone --branch build-week-judge-demo-2026 --depth 1 https://github.com/agentcarlosian/NMLT.git
    cd NMLT
    cargo build -p nmlt-cli --release
    ./judge-demo.sh --nmlt target/release/nmlt

Source-build requirements: Rust 1.94.0 through rustup, Python 3.11 or newer,
Bash, GNU Make, and coreutils.

The judge path does not require Lean, TLC, Quint, P, Node, or network access.
Those tools are used only by separate repository research and comparison
gates. The primary repository gate is make ci. The separate Lean/metatheory
gate has its own pinned toolchain and is not required to build or run the CLI.

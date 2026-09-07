.PHONY: help fmt fmt-check check lint test behavior-fixtures behavior-artifact public-surface metatheory nanoda finite-parity execution r0-baseline-tests r0-baselines r2-jobs r2-async r2-lean ci reproduce

R0_LEAN_TOOLCHAIN = $(strip $(shell cat mechanization/lean/lean-toolchain))
R0_LEAN_COMMAND_JSON ?= ["elan","run","leanprover/lean4:$(R0_LEAN_TOOLCHAIN)","lean"]
R2_LEAN_BIN ?= $(shell elan run leanprover/lean4:$(R0_LEAN_TOOLCHAIN) lean --print-prefix)/bin/lean

help:
	@echo "NMLT language-and-mathematics targets"
	@echo "  fmt               Format Rust sources"
	@echo "  fmt-check         Verify Rust formatting"
	@echo "  check             Type-check the Rust workspace"
	@echo "  lint              Run Clippy with warnings denied"
	@echo "  test              Run all Rust tests"
	@echo "  behavior-fixtures Check the positive and boundary-specific source fixtures"
	@echo "  behavior-artifact Reproduce and non-verifyingly explore behavior-core-v1"
	@echo "  public-surface    Check public links, trust inventory, and repository hygiene"
	@echo "  metatheory        Build Lean, audit axioms, and decode the matching-source artifact"
	@echo "  nanoda            Independently check all NMLT Lean declarations"
	@echo "  finite-parity     Compare the frozen Bool/Unit/enum graph with Lean"
	@echo "  execution         Reproduce and Lean-check v2 paths and resource graphs"
	@echo "  r0-baseline-tests Test the deterministic reference-workflow harness"
	@echo "  r0-baselines      Run the three frozen Python/Lean reference workflows"
	@echo "  r2-jobs           Exercise the executable-only local job subprocess prototype"
	@echo "  r2-async          Exercise asynchronous worker control and snapshot replay"
	@echo "  r2-lean           Exercise the pinned Lean adapter and async control"
	@echo "  ci                Run the Rust language gate"
	@echo "  reproduce         Run the complete Rust and Lean gate"

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

check:
	cargo check --workspace --all-targets

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace --all-targets

behavior-fixtures:
	cargo test -p nmlt-compile --test behavior_slice

behavior-artifact:
	@artifact="$$(mktemp)"; trap 'rm -f "$$artifact"' EXIT; \
		cargo run --quiet -p nmlt-cli -- elaborate examples/pivot/visible_resource_sync.nmlt --emit-core "$$artifact"; \
		cmp examples/pivot/visible_resource_sync.behavior-core-v1.json "$$artifact"; \
		cargo run --quiet -p nmlt-cli -- explore --behavior ConcreteNetwork --max-states 8 "$$artifact" | \
		grep -F "permit: ConcreteSender -> Receiver"

public-surface:
	python3 tools/check_public_surface.py

metatheory:
	./tools/check_metatheory.sh

nanoda:
	./tools/check_nanoda.sh mechanization/lean NMLT

finite-parity:
	bash tools/check_finite_parity.sh

execution:
	bash tools/check_execution.sh

r0-baseline-tests:
	python3 -m unittest discover -s tests/baselines -v

r0-baselines:
	@set -eu; mkdir -p target/r0-baselines; \
		output_dir="$$(mktemp -d target/r0-baselines/run.XXXXXX)"; \
		python3 tools/baselines/run_baselines.py --output-dir "$$output_dir" --lean-command-json '$(R0_LEAN_COMMAND_JSON)'

r2-jobs:
	python3 tools/check_job_runtime.py

r2-async:
	python3 tools/check_async_jobs.py

r2-lean:
	python3 tools/check_async_jobs.py --lean-bin "$(R2_LEAN_BIN)"

ci: fmt-check check lint test behavior-artifact public-surface r0-baseline-tests r2-jobs r2-async

reproduce: ci metatheory nanoda finite-parity execution r0-baselines r2-lean

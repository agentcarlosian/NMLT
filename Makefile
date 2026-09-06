.PHONY: help fmt fmt-check check lint test behavior-fixtures behavior-artifact public-surface metatheory nanoda r0-baseline-tests r0-baselines ci reproduce

R0_LEAN_TOOLCHAIN = $(strip $(shell cat mechanization/lean/lean-toolchain))
R0_LEAN_COMMAND_JSON ?= ["elan","run","leanprover/lean4:$(R0_LEAN_TOOLCHAIN)","lean"]

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
	@echo "  r0-baseline-tests Test the deterministic reference-workflow harness"
	@echo "  r0-baselines      Run the three frozen Python/Lean reference workflows"
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

r0-baseline-tests:
	python3 -m unittest discover -s tests/baselines -v

r0-baselines:
	@set -eu; mkdir -p target/r0-baselines; \
		output_dir="$$(mktemp -d target/r0-baselines/run.XXXXXX)"; \
		python3 tools/baselines/run_baselines.py --output-dir "$$output_dir" --lean-command-json '$(R0_LEAN_COMMAND_JSON)'

ci: fmt-check check lint test behavior-artifact public-surface r0-baseline-tests

reproduce: ci metatheory nanoda r0-baselines

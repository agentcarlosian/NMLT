.PHONY: help fmt fmt-check check lint test corpus benchmarks model-reports temporal-evidence multi-engine-evidence agentic-evidence graded-evidence evidence examples comparisons metatheory ci

help:
	@echo "NMLT development targets"
	@echo "  fmt        Format Rust sources"
	@echo "  fmt-check  Verify Rust formatting"
	@echo "  check      Type-check the Rust workspace"
	@echo "  lint       Run Clippy with warnings denied"
	@echo "  test       Run all Rust tests"
	@echo "  corpus     Verify frozen canonical source identities"
	@echo "  benchmarks Validate frozen benchmark identities and controls"
	@echo "  model-reports Reproduce source-bound explicit-state results"
	@echo "  temporal-evidence Reproduce Phase 4 lasso/refinement/runtime evidence"
	@echo "  multi-engine-evidence Reproduce Phase 5 checked composition evidence"
	@echo "  agentic-evidence Reproduce Phase 6 authority/runtime artifact graph"
	@echo "  graded-evidence Reproduce Phase 7 resource-grade evidence"
	@echo "  evidence   Reproduce canonical provider evidence manifests"
	@echo "  examples   Structurally check all canonical NMLT fixtures"
	@echo "  comparisons Validate NMLT and Quint; optionally TLC and P"
	@echo "  metatheory Build the pinned Lean kernel artifacts"
	@echo "  ci         Run the complete local CI gate"

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

corpus:
	python3 tools/canonical_examples.py

benchmarks:
	python3 tools/validate_benchmark_integrity.py --self-test

model-reports:
	python3 tools/check_model_reports.py

temporal-evidence:
	python3 tools/check_temporal_evidence.py

multi-engine-evidence:
	python3 tools/check_multi_engine_evidence.py

agentic-evidence:
	python3 tools/check_phase6_evidence.py

graded-evidence:
	python3 tools/check_graded_evidence.py

evidence:
	python3 tools/check_evidence.py

examples:
	@for source in $$(find examples -name '*.nmlt' -type f | sort); do \
		cargo run --quiet -p nmlt-cli -- check "$$source"; \
	done

comparisons:
	./tools/validate_comparisons.sh

metatheory:
	./tools/check_metatheory.sh

ci: fmt-check check lint test corpus benchmarks model-reports temporal-evidence multi-engine-evidence agentic-evidence graded-evidence evidence examples comparisons

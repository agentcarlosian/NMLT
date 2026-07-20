.PHONY: help fmt fmt-check check lint test corpus benchmarks model-reports temporal-evidence multi-engine-evidence agentic-evidence graded-evidence open-composition-evidence open-refinement-evidence open-congruence-evidence evidence examples comparisons correspondence m9-audit metatheory ci reproduce

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
	@echo "  open-composition-evidence Check M11 theorem/source/axiom bindings"
	@echo "  open-refinement-evidence Check M11-001b contract/refinement bindings"
	@echo "  open-congruence-evidence Check M11-001c two-sided finite bindings"
	@echo "  evidence   Reproduce canonical provider evidence manifests"
	@echo "  examples   Structurally check all canonical NMLT fixtures"
	@echo "  comparisons Validate NMLT and Quint; optionally TLC and P"
	@echo "  correspondence Check shared Rust/Lean M9 vectors"
	@echo "  m9-audit   Reproduce the integrated M9 vertical slice"
	@echo "  metatheory Build the pinned Lean kernel artifacts"
	@echo "  ci         Run the complete local CI gate"
	@echo "  reproduce  Run local CI plus the pinned Lean metatheory gate"

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

open-composition-evidence:
	python3 tools/check_open_composition_evidence.py

open-refinement-evidence:
	python3 tools/check_open_refinement_evidence.py

open-congruence-evidence:
	python3 tools/check_open_congruence_evidence.py
	python3 tools/check_m11_congruence_correspondence.py

evidence:
	python3 tools/check_evidence.py

examples:
	@for source in $$(find examples -name '*.nmlt' -type f | sort); do \
		cargo run --quiet -p nmlt-cli -- check "$$source"; \
	done

comparisons:
	./tools/validate_comparisons.sh

correspondence:
	python3 tools/check_m9_correspondence.py

m9-audit:
	python3 tools/check_m9_vertical_slice.py

metatheory:
	./tools/check_metatheory.sh

ci: fmt-check check lint test corpus benchmarks model-reports temporal-evidence multi-engine-evidence agentic-evidence graded-evidence open-composition-evidence open-refinement-evidence open-congruence-evidence evidence examples comparisons correspondence m9-audit

reproduce: ci metatheory

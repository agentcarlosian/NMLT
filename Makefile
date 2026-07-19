.PHONY: help fmt fmt-check check lint test corpus examples comparisons ci

help:
	@echo "NMLT development targets"
	@echo "  fmt        Format Rust sources"
	@echo "  fmt-check  Verify Rust formatting"
	@echo "  check      Type-check the Rust workspace"
	@echo "  lint       Run Clippy with warnings denied"
	@echo "  test       Run all Rust tests"
	@echo "  corpus     Verify frozen canonical source identities"
	@echo "  examples   Structurally check all canonical NMLT fixtures"
	@echo "  comparisons Validate NMLT and Quint; optionally TLC and P"
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

examples:
	@for source in $$(find examples -name '*.nmlt' -type f | sort); do \
		cargo run --quiet -p nmlt-cli -- check "$$source"; \
	done

comparisons:
	./tools/validate_comparisons.sh

ci: fmt-check check lint test corpus examples comparisons

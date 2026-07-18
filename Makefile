.PHONY: help fmt fmt-check check lint test examples ci

help:
	@echo "NMLT development targets"
	@echo "  fmt        Format Rust sources"
	@echo "  fmt-check  Verify Rust formatting"
	@echo "  check      Type-check the Rust workspace"
	@echo "  lint       Run Clippy with warnings denied"
	@echo "  test       Run all Rust tests"
	@echo "  examples   Structurally check NMLT design fixtures"
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

examples:
	cargo run --quiet -p nmlt-cli -- check examples/technicus/provider_attempt.nmlt
	cargo run --quiet -p nmlt-cli -- check examples/hyperbook/one_bit_clock.nmlt
	cargo run --quiet -p nmlt-cli -- check examples/agents/trust_chain.nmlt

ci: fmt-check check lint test examples

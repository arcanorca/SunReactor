.PHONY: fmt fmt-check clippy test check ci

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::dbg_macro -D clippy::todo

test:
	cargo test --workspace --all-targets --all-features

check:
	cargo check --workspace --all-targets --all-features

ci:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::dbg_macro -D clippy::todo
	cargo test --workspace --all-targets --all-features

.PHONY: build release install test lint fmt check clean

build:
	cargo build

release:
	cargo build --release

install: release
	cargo install --path .

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

check: fmt lint test build
	@echo "All checks passed."

clean:
	cargo clean

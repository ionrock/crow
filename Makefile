.PHONY: build release install install-plugin uninstall-plugin test lint fmt check clean

build:
	cargo build

release:
	cargo build --release

install: release
	cargo install --path .

install-plugin: install
	crow install-plugin

uninstall-plugin:
	crow install-plugin --uninstall

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

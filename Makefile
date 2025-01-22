SHELL := bash

.PHONY: build_and_run
build_and_run: build run

.PHONY: run
run:
	time ./target/release/bash-helper
.PHONY: run-debug
run-debug:
	./target/debug/bash-helper

.PHONY: build
build:
	cargo build --future-incompat-report --release
.PHONY: build-debug
build-debug:
	RUSTFLAGS="--cfg debug" cargo build --future-incompat-report

.PHONY: build-cross-x86_64
build-cross-x86_64:
	cargo build --target x86_64-unknown-linux-gnu --release
.PHONY: build-cross-aarch64
build-cross-aarch64:
	cargo build --target aarch64-unknown-linux-gnu --release


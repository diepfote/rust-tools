SHELL := bash

.PHONY: build_and_run
build_and_run: build run

.PHONY: debug_build_and_run
debug_build_and_run: build-debug run-debug

.PHONY: run
run:
	../run.sh target/release
.PHONY: run-debug
run-debug:
	../run.sh target/debug

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


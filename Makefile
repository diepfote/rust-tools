SHELL := bash

.PHONY: build run build_and_run build-cross-x86_64 build-cross-aarch64

build_and_run: build run

run:
	./target/release/bash-helper

build:
	cargo build --future-incompat-report --release

build-cross-x86_64:
	cargo build --target x86_64-unknown-linux-gnu --release
build-cross-aarch64:
	cargo build --target aarch64-unknown-linux-gnu --release

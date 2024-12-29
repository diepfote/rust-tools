SHELL := bash

.PHONY: build run build_and_run

build_and_run: build run

run:
	./target/release/bash-helper

build:
	cargo build --future-incompat-report --release

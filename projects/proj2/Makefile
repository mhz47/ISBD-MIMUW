.PHONY: all lib viewer generator test test-unit test-integration


all: lib viewer generator

lib:
	cargo build --release --lib

viewer:
	cargo build --release --bin isbd-viewer

generator:
	cargo build --release --bin isbd-generator


test: test-unit test-integration

test-unit:
	cargo test --test unit

test-integration:
	cargo test --test integration

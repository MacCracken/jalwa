.PHONY: check fmt clippy test bench build clean

check: fmt clippy test

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace -- -D warnings

test:
	cargo test --workspace

bench:
	./scripts/run-benchmarks.sh

build:
	cargo build --workspace

clean:
	cargo clean

.PHONY: build install clean test lint

build:
	cargo build --release

install: build
	@scripts/install.sh

test:
	cargo test --all

lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

clean:
	cargo clean
	rm -rf ~/.blue

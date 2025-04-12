.PHONY: build install clean test lint build-web

build:
	cargo build --release

install: build
	@scripts/install-uv.sh

build-web:
	@scripts/build-web.sh

install-web: build-web
	@echo "Web client installed to ~/.uv/assets/web"

test:
	cargo test --all

lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

clean:
	cargo clean
	rm -rf ~/.uv

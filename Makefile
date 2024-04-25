.DEFAULT_GOAL := build
.PHONY:build

release-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-gnu --release

build-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-gnu

release:
	cargo build --release

build:
	cargo build

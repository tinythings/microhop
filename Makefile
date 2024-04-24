.DEFAULT_GOAL := build
.PHONY:build

release:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-gnu --release

build:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-gnu

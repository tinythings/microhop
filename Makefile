.DEFAULT_GOAL := build
.PHONY:build microhop-release-static microhop-debug-static microgen-release microgen-debug _reset_placeholder

ARCH := $(shell uname -p)

microhop-release-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -p microhop --target $(ARCH)-unknown-linux-gnu --release

microhop-debug-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -p microhop --target $(ARCH)-unknown-linux-gnu

microgen-release:
	cargo build -p microgen --release

microgen-debug:
	cargo build -p microgen

_reset_placeholder:
	@printf "Restoring placeholders\n"
	@echo "This is only a placeholder" > microgen/src/microhop

build-debug:
	@printf "Building Microhop (debug)\n"
	@$(MAKE) microhop-debug-static

	cp target/$(ARCH)-unknown-linux-gnu/debug/microhop microgen/src

	@printf "Building Microgen\n"
	@$(MAKE) microgen-debug
	@$(MAKE) _reset_placeholder

	@printf "\n\nDone. Debug version is built for you in target/debug\n\n"

build-release:
	@printf "Building Microhop (release)\n"
	@$(MAKE) microhop-release-static

	cp target/$(ARCH)-unknown-linux-gnu/release/microhop microgen/src

	@printf "Building Microgen\n"
	@$(MAKE) microgen-release
	@$(MAKE) _reset_placeholder

	@printf "\n\nDone. Debug version is built for you in target/release\n\n"

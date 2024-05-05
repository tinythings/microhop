.DEFAULT_GOAL := build
.PHONY:build microhop-release-static microhop-debug-static microgen-release microgen-debug _shift_placeholder _unshift_placeholder

microhop-release-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -p microhop --target x86_64-unknown-linux-gnu --release

microhop-debug-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -p microhop --target x86_64-unknown-linux-gnu

microgen-release:
	cargo build -p microgen --release

microgen-debug:
	cargo build -p microgen

_shift_placeholder:
	@printf "Shifting placeholders\n"
	cp microgen/src/microhop microgen/src/microhop.temp

_unshift_placeholder:
	@printf "Restoring placeholders\n"
	mv microgen/src/microhop.temp microgen/src/microhop

build-debug:
	@printf "Building Microhop (debug)\n"
	@$(MAKE) microhop-debug-static
	@$(MAKE) _shift_placeholder

	cp target/x86_64-unknown-linux-gnu/debug/microhop microgen/src

	@printf "Building Microgen\n"
	@$(MAKE) microgen-debug
	@$(MAKE) _unshift_placeholder

	@printf "\n\nDone. Debug version is built for you in target/debug\n\n"

build-release:
	@printf "Building Microhop (release)\n"
	@$(MAKE) microhop-release-static
	@$(MAKE) _shift_placeholder

	cp target/x86_64-unknown-linux-gnu/release/microhop microgen/src

	@printf "Building Microgen\n"
	@ls -lah microgen/src
	@$(MAKE) microgen-release
	@$(MAKE) _unshift_placeholder

	@printf "\n\nDone. Debug version is built for you in target/release\n\n"

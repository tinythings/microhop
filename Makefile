.DEFAULT_GOAL := build
.PHONY:build microhop-release-static microhop-debug-static microgen-release microgen-debug _reset_placeholder

ARCH := $(shell uname -p)
ARC_VERSION := $(shell cat src/microhop.rs | grep 'static VERSION' | sed -e 's/.*=//g' -e 's/[" ;]//g')
ARC_NAME := microhop-${ARC_VERSION}

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

tar:
	rm -rf package/${ARC_NAME}
	cargo vendor
	mkdir -p package/${ARC_NAME}/.cargo
	cp .vendor.toml package/${ARC_NAME}/.cargo/config.toml

	cp LICENSE package/${ARC_NAME}
	cp README.md package/${ARC_NAME}
	cp Cargo.lock package/${ARC_NAME}
	cp Cargo.toml package/${ARC_NAME}
	cp Makefile package/${ARC_NAME}
	cp -a microgen package/${ARC_NAME}
	cp -a profile package/${ARC_NAME}
	cp -a src package/${ARC_NAME}
	cp -a vendor package/${ARC_NAME}

	# Cleanup. Also https://github.com/rust-lang/cargo/issues/7058
	find package/${ARC_NAME} -type d -wholename "*/target" -prune -exec rm -rf {} \;
	find package/${ARC_NAME} -type d -wholename "*/vendor/winapi*" -prune -exec \
		rm -rf {}/src \; -exec mkdir -p {}/src \; -exec touch {}/src/lib.rs \; -exec rm -rf {}/lib \;
	find package/${ARC_NAME} -type d -wholename "*/vendor/windows*" -prune -exec \
		rm -rf {}/src \; -exec mkdir -p {}/src \;  -exec touch {}/src/lib.rs \; -exec rm -rf {}/lib \;
	rm -rf package/${ARC_NAME}/vendor/web-sys/src/*
	rm -rf package/${ARC_NAME}/vendor/web-sys/webidls
	mkdir -p package/${ARC_NAME}/vendor/web-sys/src
	touch package/${ARC_NAME}/vendor/web-sys/src/lib.rs

	# Tar the source
	tar -C package -czvf package/${ARC_NAME}.tar.gz ${ARC_NAME}
	rm -rf package/${ARC_NAME}
	rm -rf vendor

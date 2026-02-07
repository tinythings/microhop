.DEFAULT_GOAL := build
.PHONY:build microhop-release-static microhop-debug-static microgen-release microgen-debug _reset_placeholder e2fsprogs-static

ARCH := $(shell uname -m)
ARC_VERSION := $(shell cat src/microhop.rs | grep 'static VERSION' | sed -e 's/.*=//g' -e 's/[" ;]//g')
ARC_NAME := microhop-${ARC_VERSION}

E2FSPROGS_URL := https://github.com/phodina/static-tools/releases/download/v1.0/e2fsprogs-static-v1.0.tar.gz
E2FSPROGS_TARBALL := e2fsprogs-static-v1.0.tar.gz
E2FSPROGS_SHA256 := 5a7edc1e509be23075d8f7ee5c05008c5f8915a63c66c9d7da5187dcca522c7a
E2FSPROGS_DIR := third_party/e2fsprogs-static-v1.0
E2FSCK_BINARY_PATH := $(E2FSPROGS_DIR)/bin/e2fsck

MICROHOP_BINARY_PATH_DEBUG := target/$(ARCH)-unknown-linux-gnu/debug/microhop
MICROHOP_BINARY_PATH_RELEASE := target/$(ARCH)-unknown-linux-gnu/release/microhop

microhop-release-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -p microhop --target $(ARCH)-unknown-linux-gnu --release

microhop-debug-static:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -p microhop --target $(ARCH)-unknown-linux-gnu

microgen-release:
	cargo build -p microgen --release

microgen-debug:
	cargo build -p microgen

e2fsprogs-static:
	@printf "Fetching e2fsprogs static tools\n"
	@mkdir -p $(E2FSPROGS_DIR)
	@[ -f $(E2FSPROGS_TARBALL) ] || curl -L -o $(E2FSPROGS_TARBALL) $(E2FSPROGS_URL)
	@printf "Verifying sha256\n"
	@echo "$(E2FSPROGS_SHA256)  $(E2FSPROGS_TARBALL)" | sha256sum -c -
	@printf "Extracting\n"
	@tar -xzf $(E2FSPROGS_TARBALL) -C $(E2FSPROGS_DIR)

_reset_placeholder:
	@printf "Restoring placeholders\n"
	@echo "This is only a placeholder" > microgen/src/microhop

build-debug: MICROHOP_BINARY_PATH=$(MICROHOP_BINARY_PATH_DEBUG)
build-debug: e2fsprogs-static
	@printf "Building Microhop (debug)\n"
	@$(MAKE) microhop-debug-static

	cp $(MICROHOP_BINARY_PATH) microgen/src

	@printf "Building Microgen\n"
	@$(MAKE) microgen-debug
	@$(MAKE) _reset_placeholder

	@printf "\n\nDone. Debug version is built for you in target/debug\n\n"

build-release: MICROHOP_BINARY_PATH=$(MICROHOP_BINARY_PATH_RELEASE)
build-release: e2fsprogs-static
	@printf "Building Microhop (release)\n"
	@$(MAKE) microhop-release-static

	cp $(MICROHOP_BINARY_PATH) microgen/src

	@printf "Building Microgen\n"
	@$(MAKE) microgen-release
	@$(MAKE) _reset_placeholder

	@printf "\n\nDone. Debug version is built for you in target/release\n\n"

clean:
	cargo clean

test:
	#cargo nextest run --workspace

check:
	cargo clippy --all -- -Dwarnings -Aunused-variables -Adead-code

fix:
	cargo clippy --fix --allow-dirty --allow-staged --all

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

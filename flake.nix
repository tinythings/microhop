{
  description = "microhop - Minimal initramfs /init binary and generator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    {
      nixConfig = {
        extra-substituters = [ "https://mobile-nixos-next.cachix.org" ];
        extra-trusted-public-keys = [ "mobile-nixos-next.cachix.org-1:tPehb3T4X8DKn3sVsOUu010Tw8MFElOSizi2w3AQc5Y=" ];
      };
    } // flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        e2fsprogsNoTest = pkgs.pkgsStatic.e2fsprogs.overrideAttrs (old: {
          doCheck = false;
        });

        microgenCargoToml = builtins.fromTOML (builtins.readFile ./microgen/Cargo.toml);
        version = microgenCargoToml.package.version;

        # Internal (not exported) microhop package used by microgen.
        microhopPkg = pkgs.pkgsStatic.rustPlatform.buildRustPackage rec {
          pname = "microhop";
          inherit version;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "kmoddep-0.1.5" = "sha256-8Q2cL2YYpItJ/aIwiqhT3iAsMwzBemAem0deKHRxXDs=";
            };
          };

          nativeBuildInputs = with pkgs.pkgsStatic; [
            pkg-config
            rustPlatform.bindgenHook
          ];

          buildInputs = with pkgs.pkgsStatic; [
            util-linuxMinimal
            e2fsprogsNoTest.dev
          ];

          buildType = "release";

          cargoBuildFlags = [ "-p" "microhop" "--features" "nixos" ];

          doCheck = false;

          stripAllList = [ "bin" ];

          meta = with pkgs.lib; {
            description = "Minimal initramfs /init binary";
            homepage = "https://github.com/tinythings/microhop";
            license = licenses.asl20;
            maintainers = [];
            platforms = [ "aarch64-linux" "x86_64-linux" ];
          };
        };

        bootComponents = if system == "aarch64-linux" then {
          kernel = import ./nixos/kernel.nix { inherit pkgs; };
          u-boot = import ./nixos/u-boot.nix { inherit pkgs; };

          microhopConfig = import ./nixos/microhop-config.nix { inherit pkgs; };

          nixos-rootfs = import ./nixos/nixos-rootfs.nix {
            inherit pkgs nixpkgs;
          };

          # NixOS systemd rootfs as ext4 image
          nixos-systemd-rootfs = import ./nixos/nixos-systemd/rootfs.nix {
            inherit pkgs nixpkgs;
          };

          # Produce a corrupted ext4 image (external overlay) from the flake
          nixos-rootfs-img = pkgs.runCommand "test-rootfs-img" {
            nativeBuildInputs = with pkgs; [ coreutils e2fsprogsNoTest ];
          } ''
            mkdir -p $out
            TMP_EXT4="$out/test-rootfs.img"
            ${pkgs.coreutils}/bin/truncate -s 64M "$TMP_EXT4"
            ${e2fsprogsNoTest}/bin/mkfs.ext4 -F -L test-rootfs "$TMP_EXT4"
            # Corrupt metadata
            ${pkgs.coreutils}/bin/dd if=/dev/urandom of="$TMP_EXT4" bs=1024 count=2 seek=1 conv=notrunc
            # Corrupt random blocks
            ${pkgs.coreutils}/bin/dd if=/dev/urandom of="$TMP_EXT4" bs=4096 count=20 seek=100 conv=notrunc
            # Corrupt inode tables
            ${pkgs.coreutils}/bin/dd if=/dev/urandom of="$TMP_EXT4" bs=4096 count=5 seek=200 conv=notrunc
          '';
        } else {};
      in
      {
          packages = {
          microhop = pkgs.pkgsStatic.rustPlatform.buildRustPackage rec {
            pname = "microhop";
            inherit version;

            src = ./.;

            GIT_COMMIT = self.rev or self.dirtyRev or "unknown";

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "kmoddep-0.1.5" = "sha256-8Q2cL2YYpItJ/aIwiqhT3iAsMwzBemAem0deKHRxXDs=";
              };
            };

            nativeBuildInputs = (with pkgs.pkgsStatic; [
              pkg-config
              git # To embed the build commit ID to microhop.conf 
              rustPlatform.bindgenHook
            ]) ++ [
              microhopPkg
            ] ++ [ e2fsprogsNoTest ];

            buildInputs = with pkgs.pkgsStatic; [
              util-linuxMinimal
            ];

            buildType = "release";

            cargoBuildFlags = [ "-p" "microgen" "--features" "nixos" ];

            # Set environment variable to point to microhop and e2fsck binaries for include_bytes!()
            # The nativeBuildInputs ensures these are available at build time
            MICROHOP_BINARY_PATH = "${microhopPkg}/bin/microhop";
            E2FSCK_BINARY_PATH = "${e2fsprogsNoTest}/bin/e2fsck";

            doCheck = false;

            meta = with pkgs.lib; {
              description = "Initramfs generator tool for microhop";
              homepage = "https://github.com/tinythings/microhop";
              license = licenses.asl20;
              maintainers = [];
              platforms = [ "aarch64-linux" "x86_64-linux" ];
            };
          };

          default = self.packages.${system}.microhop;
        } // (if system == "aarch64-linux" then {

          kernel = bootComponents.kernel;
          u-boot = bootComponents.u-boot;
          nixos-rootfs = bootComponents.nixos-rootfs;
          nixos-systemd-rootfs = bootComponents.nixos-systemd-rootfs;

          initramfs-microgen = import ./nixos/initramfs-microgen.nix {
            inherit pkgs;
            kernel = bootComponents.kernel;
            microgen = self.packages.${system}.microhop;
            microhop = microhopPkg;
            microhopConfig = bootComponents.microhopConfig;
          };

          # NixOS systemd-specific initramfs
          nixos-systemd-initramfs = import ./nixos/initramfs-microgen.nix {
            inherit pkgs;
            kernel = bootComponents.kernel;
            microgen = self.packages.${system}.microhop;
            microhop = microhopPkg;
            microhopConfig = import ./nixos/nixos-systemd/microhop-config.nix { inherit pkgs; };
          };

          nixos-example = pkgs.writeScriptBin "boot-qemu-overlayfs-musl" ''
            #!${pkgs.bash}/bin/bash

            UBOOT="${bootComponents.u-boot}/u-boot.bin"
            KERNEL="${bootComponents.kernel}/Image"
            INITRD="${self.packages.${system}.initramfs-microgen}/initrd"
            SQUASHFS="${bootComponents.nixos-rootfs}/rootfs.squashfs"

            # Use the corrupted ext4 image produced by the flake as the overlay
            OVERLAY_IMG="${bootComponents.nixos-rootfs-img}/test-rootfs.img"

            # Copy overlay image out of the Nix store to a writable temp file
            TMP_OVERLAY=$(mktemp /tmp/test-rootfs.img.XXXX)
            cp -f "$OVERLAY_IMG" "$TMP_OVERLAY"
            chmod 644 "$TMP_OVERLAY"
            trap 'rm -f "$TMP_OVERLAY"' EXIT

            echo "=========================================="
            echo "  Microhop Boot Flow"
            echo "=========================================="
            echo "Booting with musl-based rootfs and overlayfs..."
            echo ""
            echo "U-Boot:         $UBOOT"
            echo "Kernel:         $KERNEL"
            echo "Initrd:         $INITRD"
            echo "Rootfs:         $SQUASHFS"
            echo "Overlay (store): $OVERLAY_IMG"
            echo "Overlay (tmp):   $TMP_OVERLAY"
            echo ""
            echo "Boot flow: U-Boot -> Kernel -> Microhop initramfs -> NixOS musl rootfs"
            echo "=========================================="
            echo ""
              ${pkgs.qemu}/bin/qemu-system-aarch64 \
                -M virt \
                -cpu cortex-a57 \
                -m 1024M \
                -smp 2 \
                -bios "$UBOOT" \
                -kernel "$KERNEL" \
                -initrd "$INITRD" \
                -drive file="$SQUASHFS",if=none,format=raw,readonly=on,id=hd0 \
                -device virtio-blk-device,drive=hd0 \
                -drive file="$TMP_OVERLAY",if=none,format=raw,id=hd1 \
                -device virtio-blk-device,drive=hd1 \
                -append "console=ttyAMA0 root=/dev/null" \
                -nographic

          '';

          nixos-systemd-example = pkgs.writeScriptBin "boot-nixos-systemd" ''
            #!${pkgs.bash}/bin/bash

            KERNEL="${bootComponents.kernel}/Image"
            INITRD="${self.packages.${system}.nixos-systemd-initramfs}/initrd"
            EXT4_ROOTFS="${bootComponents.nixos-systemd-rootfs}"

            # Copy ext4 image out of the Nix store to a writable temp file (QEMU needs write access)
            TMP_EXT4=$(mktemp /tmp/nixos-systemd-rootfs.ext4.XXXX)
            cp -f "$EXT4_ROOTFS" "$TMP_EXT4"
            chmod 644 "$TMP_EXT4"
            trap 'rm -f "$TMP_EXT4"' EXIT

            echo "=========================================="
            echo "  NixOS Systemd Boot Flow"
            echo "=========================================="
            echo "Booting NixOS with systemd init and microhop initramfs..."
            echo ""
            echo "Kernel:         $KERNEL"
            echo "Initrd:         $INITRD"
            echo "Rootfs (store): $EXT4_ROOTFS"
            echo "Rootfs (temp):  $TMP_EXT4"
            echo ""
            echo "Boot flow: Kernel -> Microhop initramfs -> NixOS systemd"
            echo "=========================================="
            echo ""

            echo "Starting QEMU with pre-built ext4 rootfs..."
            echo ""

            exec ${pkgs.qemu}/bin/qemu-system-aarch64 \
              -machine virt \
              -cpu cortex-a57 \
              -smp 2 \
              -m 1024 \
              -kernel "$KERNEL" \
              -initrd "$INITRD" \
              -drive file="$TMP_EXT4",if=none,format=raw,id=hd0 \
              -device virtio-blk-device,drive=hd0 \
              -append "console=ttyAMA0,115200 root=/dev/vda rootfstype=ext4 rw init=/nix/var/nix/profiles/system" \
              -nographic \
              -serial mon:stdio \
              "$@"
          '';
        } else {}) // (if system == "aarch64-linux" then {

          nixos-systemd = let
            nixosSystem = nixpkgs.lib.nixosSystem {
              system = "aarch64-linux";
              modules = [ 
                {
                  _module.args = {
                    inherit nixpkgs;
                    microhop = microhopPkg;
                    microgen = self.packages.${system}.microhop;
                  };
                }
                ./nixos/nixos-systemd/system.nix 
              ];
            };
          in nixosSystem.config.system.build.qemu-system;

        } else {});

        apps = if system == "aarch64-linux" then {
          nixos-example = {
            type = "app";
            program = "${self.packages.${system}.nixos-example}/bin/boot-qemu-overlayfs-musl";
          };
          nixos-systemd = {
            type = "app";
            program = "${self.packages.${system}.nixos-systemd-example}/bin/boot-nixos-systemd";
          };
          default = {
            type = "app";
            program = "${self.packages.${system}.nixos-example}/bin/boot-qemu-overlayfs-musl";
          };
        } else {};


        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            gnumake
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            util-linux
            llvmPackages_20.libclang
            llvmPackages_20.clang
          ];

          shellHook = ''
            export LIBCLANG_PATH=${pkgs.libclang.lib}/lib
            export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=${pkgs.glibc.dev} -I${pkgs.util-linux.dev}/include"
            export MICROHOP_BINARY_PATH=${microhopPkg}/bin/microhop
            export E2FSCK_BINARY_PATH=${e2fsprogsNoTest}/bin/e2fsck
          '';
        };
      }
    );
}

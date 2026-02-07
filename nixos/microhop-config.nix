{ pkgs ? import <nixpkgs> {} }:

pkgs.writeText "microhop.conf" ''
  # Microhop configuration for QEMU aarch64 with overlayfs over squashfs
  # Kernel modules to load
  # Note: virtio_blk, virtio_mmio, squashfs, and overlay are built-in (=y) in kernel.nix
  modules: []

  # Devices mounting
  # Mount the squashfs rootfs from /dev/vdb (read-only) - this is the base layer
  # The mountpoint should be empty to mount directly to sysroot
  disks:
    /dev/vdb: squashfs,,ro

  # Execute systemd/init after switching root
  # Note: After chroot, /sysroot becomes /, so the init is at /init
  init: /init

  # Temporary sysroot location
  sysroot: /sysroot

  # Overlayfs configuration
  # /dev/vda is the ext4 partition for the overlay upper/work dirs
  overlayfs:
    device: /dev/vda
    upper: upper
    workdir: work

  # Mask kernel cmdline parameters so bootloader-provided root= is ignored
  mask_cmdline:
    - root

  # Debug log output
  log: debug
''

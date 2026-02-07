# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-24
### Added
- Nix flake support for building packages
  (support for `aarch64-linux` and `x86_64-linux` platforms)
- GitHub workflow for building Nix packages across multiple platforms
  (darwin not supported due to 'nixbld' user cleanup issue)
- Add support for `overlayfs` to support read-only rootfs
- Handle missing kernel modules gracefully in runtime
- Add kernel config validation
- Add arguments to specify and validate the filesystem
  and block device for rootfs during creation
- Support block devices using also labels
- Add Nix definitions for u-boot and latest stable kernel
- Add support to run qemu-system-aarch64
- Add example minimal Nixos system to boot

## [0.1.0] - 2024-07-10

### Added
- Add analyser of a current system

## [0.0.9] - 2024-07-04

### Added
- Rework CLI into sub-commands

## [0.0.8] - 2024-07-03

### Added
- Bugfix: Default init should be `/sbin/init`
- Bugfix: dependencies ordering

## [0.0.7] - 2024-05-13

### Added
- Implements native CPIO/zstd generator of the initramfs.zst file
  (rather then rely on external tools to shell-out)

## [0.0.6] - 2024-05-07

### Added
- Added ability to mount by disk labels
- Fixed bugs for Debian family

## [0.0.5] - 2024-05-05

### Added
- TBD by @isbm

## [0.0.4] - 2024-05-02

### Added
- Mount disks by UUID
- Initial release of microhop - Minimal initramfs /init binary
- Initial release of microgen - Initramfs generator tool

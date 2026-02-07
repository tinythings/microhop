# Microhop - initramfs helper

![NixOS booting using microhop (video)](./nixos-aarch64-qemu.mp4)

You do not always need Dracut. 😉 Sometimes you want it really-really
small, tiny and completely stripped from everything. This is what
`microhop` is for: use mainline Linux Kernel straight to the point,
omitting many generic moving parts.

# Tutorial

### Overview

**Microhop** when deployed consists of one binary utility and one configuration file:
  1. `microgen` — utility, that generates your initramfs as CPIO gzipped archive
  2. `/etc/microhop.conf` — main system boot configuration, can be also your profile

The resulting `microhop` binary is the very `/init` in the `initramfs`,
containing all required functionality, such as mounting, block device
detection, root switching etc. It will appear only inside the `initramfs` CPIO archive
and is not intended to use anywhere else.

The `microgen` binary is the utility which generates the `initramfs` archive.
This archive then you will copy into the `/boot` directory of your Linux image.

### Building

> **WARNING: Do not build it directly using `cargo`, because you will get it all wrong!**

To build Microhop, first clone this repository and then just run:

	make build-release

In `./target/release` you should have a binary, called `microgen`. This is all you need.

#### Note on Dependencies

You might need to adjust your setup. For example, on Debian/Ubuntu you would need to
install the following packages:

- `libclang-dev`
- `libblkid-dev`

On openSUSE Leap `static-pie` linking is *broken*, and only `static` is available. With that in mind,
only `"thin"` LTO for the release profile is available _(unless it was fixed)_. Additionally,
you will need the following packages on openSUSE Leap:

- `libblkid-devel-static`
- `glibc-devel-static`

### Building using Nix

It's possible to build the binaries using [Nix](https://nixos.org/).

The package is statically linked and uses [musl](https://www.musl-libc.org/) due to dependency on
`util-linuxMinimal` which provides `libblkid`.

```shell
# Build the aarch64 variant
$ nix build .#packages.aarch64-linux.microhop

$ file result/bin/microhop
result/bin/microhop: ELF 64-bit LSB executable, ARM aarch64, version 1 (SYSV), statically linked, stripped

$ du -b result/bin/microhop
1186064 result/bin/microhop
```

To run the demo shown in the captured [asciinema](https://asciinema.org/) do the following:
```
# To exit QEMU type Ctrl+A X
nix run .#boot-overlayfs-musl
```

**Note:** Flake-based derivations and Cachix substitutes are available for faster,
reproducible builds; see `flake.nix` and the `nixos/` examples.

To list available nix derivations run:
```
nix flake show
```

To speed up the build there's a [Cachix](https://app.cachix.org/cache/mobile-nixos-next#search) available.

### Configuration

Configuration is also a profile. This is the basic start:

```yaml
# What kernel modules to load
# NOTE: currently one needs to load
#       far dependencies first, and then
#       the final module, otherwise it won't do. :)
modules:
  - virtio_blk
  - jbd2
  - crc16
  - mbcache
  - ext4

# Devices mounting
# NOTE: If kernel command line includes root= parameter, it will override
#       the disk configuration below. This allows bootloader to specify rootfs.
disks:
# Preferred methods (most reliable):
# By UUID (recommended):
  uuid=24e1daee-e09b-4fd5-97f3-dde8aba6ad8a: ext4,/,rw

# Optionally, define another init app, if it is not /sbin/init
# This app will be launched with PID 1 and should never quit.
init: /usr/bin/bash

# Optionally, define a temporary sysroot.
# Default: /sysroot
sysroot: /sysroot

# Optionally, set debug log output. If this option is removed, default is used.
# Choose one from:
# - debug
# - info (default)
# - quiet (errors only)
log: debug

# Optionally, enable overlayfs support
# This creates a writable layer on top of the read-only root filesystem.
# Useful for systems with read-only filesystems like squashfs or when booting from
# read-only media. Requires kernel CONFIG_OVERLAY_FS support.
#
# overlayfs:
#   device: uuid=12345678-1234-1234-1234-123456789abc  # or label=overlay-storage or /dev/vdb1
#   upper: upper      # Path on the mounted device for the upper layer
#   workdir: work     # Path on the mounted device for overlayfs work directory

# Firmware configuration
# Defines the base path for firmware files in the initramfs
# Default: /lib/firmware
firmware:
  base: /lib/firmware

# Optionally, mask (ignore) specific kernel cmdline parameters
# This is useful when the bootloader passes conflicting parameters that you want to override
# with values from this config file instead.
#
# Common use case: Android bootloaders often pass a hardcoded root=PARTUUID=... that doesn't
# match the actual filesystem. By masking "root", microhop will ignore the bootloader's
# parameter and use the disk configuration above instead.
#
# mask_cmdline:
#   - root       # Ignore root= from bootloader, use disks config instead
```

To validate the config use `microgen validate` to sanity-check your `microhop.conf` before generating an
initramfs.

Resulting configuration will just contain more modules (their dependencies). The rest will be passed through.

### Generating initramfs

Essentially, the workflow is very simple:

1. Point which kernel you want to use and where it is
2. "Press a pedal" to get a new `initramfs`
3. Wait whole 0.05 seconds and you have it. 😉

To achieve this, do the following:

1. Mount your root filesystem, which you want to update with the new `initramfs`. As an example, setup
  a device with `losetup` and then mount one of its partitions:
   ```shell
   sudo mount /dev/loop1p3 /mnt
   ```

2. Let `microgen` generate it _(NOTE: this is an example, your filenames may differ)_:

   ```shell
   sudo microgen new --root /mnt --config microhop.conf --file /mnt/boot/initrd-5.14.21-default
   ```

   This command above is analysing your root filesystem at `/mnt`, will use `microhop.conf` as a profile and will write the output CPIO archive to the path, specified by `--file` option.

   `microgen` supports embedding `e2fsck` binary to recover filesystem from errors and to include additional
firmware files or embedded binaries

3. Un-mount your image:

   ```shell
   sudo umount /mnt
   ```

That's basically it and hopefully it will even boot... 😉

## [Changelog](./CHANGELOG.md)

Feel free to checkout the changes between the releases.

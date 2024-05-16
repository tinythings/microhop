# Microhop - initramfs helper

You do not always need Dracut. 😉 Sometimes you want it really-really
small, tiny and completely stripped from everything. This is what
`microhop` is for: use mainline Linux Kernel straight to the point,
omitting many generic moving parts.

# Tutorial

### Overview

**Microhop** when deployed consists of one binary utility and one configuration file:
  1. `microgen` — dynamic utility on the host system
  2. `/etc/microhop.conf` — main system boot configuration

The resulting `microhop` binary is the very `/init` in the `initramfs`,
containing all required functionality, such as mounting, block device
detection, root switching etc. It will appear only inside the `initramfs` CPIO archive
and is not intended to use elsewhere.

The `microgen` binary is the utility which is generating the `initramfs` archive.
This archive then you will copy into the `/boot` directory of your Linux image.

### Building

Clone this repository and run:

	make build-release

In `./target/release` you should have a binary, called `microgen`. This is all you need.

#### Note on Dependencies

You might need to adjust your setup. For example, on Debian/Ubuntu you would need to
install the following packages:

- `libclang-dev`
- `libblkid-dev`

On openSUSE Leap `static-pie` linking is broken, and only `static` is available. With that in mind,
only `"thin"` LTO for the release profile is available _(unless it was fixed)_. Additionally,
you will need the following packages on openSUSE Leap:

- `libblkid-devel-static`
- `glibc-devel-static`


### Configuration

Configuration is also a profile. This is the basic start:

```yaml
# The list of kernel modules
modules:
  - virtio_blk
  - ext4

# Devices
disks:
  # /dev/vda3: ext4,/,rw
  24e1daee-e09b-4fd5-97f3-dde8aba6ad8a: ext4,/,rw

# Optionally, define a custom init app
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
```

Resulting configuration will just contain more modules (their dependencies). The rest will be passed through.

### Generating initramfs

Essentially, the workflow is very simple:

1. Point which kernel you want to use and where it is
2. "Press pedal" to get a new `initramfs`

To achieve the effect, do the following:

1. Mount your root filesystem, which you want to update with the new `initramfs`. For example:
   ```shell
   sudo mount /dev/loop1p3 /mnt
   ```

2. Let `microgen` generate it _(NOTE: this is an example, your filenames may differ)_:

   ```shell
   sudo microgen --root /mnt --config microhop.conf --file /mnt/boot/initrd-5.14.21-default
   ```

3. Un-mount your image:

   ```shell
   sudo umount /mnt
   ```

That's basically it. 😉

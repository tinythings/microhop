# Microhop - initramfs helper

You do not always need Dracut. 😉 Sometimes you want it really-really
small, tiny and completely stripped from everything. This is what
`microhop` is for: use mainline Linux Kernel straight to the point,
omitting many generic moving parts.

# Usage

**Microhop** consists of two binaries and one configuration file:
  1. `microhop` — should be compiled static
  2. `microgen` — dynamic utility on the host system
  3. `/etc/microhop.conf` — main system boot configuration

The `microhop` binary is the very `/init` in the `initramfs`,
containing all required functionality, such as mounting, block device
detection, root switching etc.

The `microgen` binary is the utility to generate the `initramfs`
archive, which will be included into the `/boot` directory of the
Linux image.

## Configuration

```yaml
# The list of kernel modules
modules:
  - virtio_blk
  - jbd2
  - crc16
  - mbcache
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

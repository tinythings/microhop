{ pkgs ? import <nixpkgs> {} }:

let
  isNative = pkgs.stdenv.hostPlatform.isAarch64;

  kernelConfig = pkgs.writeText "kernel.config" ''
    # Architecture
    CONFIG_ARM64=y
    CONFIG_64BIT=y
    CONFIG_ARCH_VIRT=y

    # General setup
    CONFIG_LOCALVERSION=""
    CONFIG_DEFAULT_HOSTNAME="qemu-aarch64"
    CONFIG_SWAP=n
    CONFIG_SYSVIPC=y
    CONFIG_POSIX_MQUEUE=y
    CONFIG_CROSS_MEMORY_ATTACH=y
    CONFIG_USELIB=y
    CONFIG_AUDIT=n
    CONFIG_IKCONFIG=n
    CONFIG_LOG_BUF_SHIFT=17
    CONFIG_PRINTK=y
    CONFIG_BUG=y
    CONFIG_ELF_CORE=y
    CONFIG_BASE_FULL=y
    CONFIG_FUTEX=y
    CONFIG_EPOLL=y
    CONFIG_SIGNALFD=y
    CONFIG_TIMERFD=y
    CONFIG_EVENTFD=y
    CONFIG_SHMEM=y
    CONFIG_AIO=y
    CONFIG_ADVISE_SYSCALLS=y
    CONFIG_EMBEDDED=n

    # Control Group support (required by systemd)
    CONFIG_CGROUPS=y
    CONFIG_CGROUP_FREEZER=y
    CONFIG_CGROUP_PIDS=y
    CONFIG_CGROUP_DEVICE=y
    CONFIG_CGROUP_CPUACCT=y
    CONFIG_CGROUP_PERF=y
    CONFIG_CGROUP_BPF=y
    CONFIG_MEMCG=y
    CONFIG_BLK_CGROUP=y

    # File handle support (required by systemd)
    CONFIG_FHANDLE=y

    # SECCOMP support (required by systemd)
    CONFIG_SECCOMP=y
    CONFIG_SECCOMP_FILTER=y

    # DMI support (required by systemd)
    CONFIG_DMIID=y

    # Kernel compression
    CONFIG_KERNEL_GZIP=n
    CONFIG_KERNEL_BZIP2=n
    CONFIG_KERNEL_LZMA=n
    CONFIG_KERNEL_XZ=n
    CONFIG_KERNEL_LZO=n
    CONFIG_KERNEL_LZ4=y

    # Kernel features
    CONFIG_SMP=y
    CONFIG_NR_CPUS=4
    CONFIG_HOTPLUG_CPU=y

    # Platform selection
    CONFIG_ARCH_VIRT=y

    # Kernel Features
    CONFIG_ARM64_PAGE_SHIFT=12
    CONFIG_ARM64_VA_BITS=39
    CONFIG_ARM64_4K_PAGES=y

    # Boot options
    CONFIG_CMDLINE="console=ttyAMA0"
    CONFIG_CMDLINE_FROM_BOOTLOADER=y

    # Virtualization
    CONFIG_VIRTUALIZATION=n

    # General architecture-dependent options
    CONFIG_HAVE_OPROFILE=y
    CONFIG_KPROBES=n
    CONFIG_JUMP_LABEL=n
    CONFIG_MODULES=y
    CONFIG_MODULE_UNLOAD=y

    # Executable file formats
    CONFIG_BINFMT_ELF=y
    CONFIG_BINFMT_SCRIPT=y
    CONFIG_COREDUMP=y

    # Networking support
    CONFIG_NET=y
    CONFIG_UNIX=y

    # Device Drivers
    CONFIG_PCI=n
    CONFIG_DEVTMPFS=y
    CONFIG_DEVTMPFS_MOUNT=y
    CONFIG_STANDALONE=y
    CONFIG_PREVENT_FIRMWARE_BUILD=y
    CONFIG_FW_LOADER=n

    # Generic Driver Options
    CONFIG_BLK_DEV=y

    # Character devices
    CONFIG_TTY=y
    CONFIG_VT=y
    CONFIG_VT_CONSOLE=y
    CONFIG_CONSOLE_TRANSLATIONS=y
    CONFIG_UNIX98_PTYS=y
    CONFIG_LEGACY_PTYS=n

    # Serial drivers
    CONFIG_SERIAL_AMBA_PL011=y
    CONFIG_SERIAL_AMBA_PL011_CONSOLE=y
    CONFIG_SERIAL_CORE=y
    CONFIG_SERIAL_CORE_CONSOLE=y

    # Input device support
    CONFIG_INPUT=y
    CONFIG_INPUT_KEYBOARD=y
    CONFIG_INPUT_MOUSE=y
    CONFIG_INPUT_EVDEV=y

    # Hardware I/O ports
    CONFIG_SERIO=y
    CONFIG_SERIO_LIBPS2=y

    # Graphics support
    CONFIG_FB=y
    CONFIG_FB_SIMPLE=y
    CONFIG_FRAMEBUFFER_CONSOLE=y
    CONFIG_DUMMY_CONSOLE=y
    CONFIG_DRM=y
    CONFIG_DRM_SIMPLEDRM=y

    # Virtio drivers
    CONFIG_VIRTIO=y
    CONFIG_VIRTIO_MMIO=y
    CONFIG_VIRTIO_BLK=y
    CONFIG_VIRTIO_CONSOLE=n

    # File systems
    CONFIG_EXT4_FS=y
    CONFIG_EXT4_USE_FOR_EXT2=y
    CONFIG_EXT4_FS_POSIX_ACL=y
    CONFIG_EXT4_FS_SECURITY=y
    CONFIG_JBD2=y
    CONFIG_FS_MBCACHE=y
    CONFIG_SQUASHFS=y
    CONFIG_SQUASHFS_ZLIB=y
    CONFIG_SQUASHFS_ZSTD=y
    CONFIG_SQUASHFS_XZ=y
    CONFIG_SQUASHFS_LZO=y
    CONFIG_SQUASHFS_LZ4=y
    CONFIG_SQUASHFS_FILE_CACHE=y
    CONFIG_SQUASHFS_FILE_DIRECT=y
    CONFIG_OVERLAY_FS=y
    CONFIG_FILE_LOCKING=y
    CONFIG_FSNOTIFY=y
    CONFIG_DNOTIFY=y
    CONFIG_INOTIFY_USER=y
    CONFIG_PROC_FS=y
    CONFIG_PROC_SYSCTL=y
    CONFIG_PROC_PAGE_MONITOR=y
    CONFIG_KERNFS=y
    CONFIG_SYSFS=y
    CONFIG_TMPFS=y
    CONFIG_TMPFS_POSIX_ACL=y
    CONFIG_TMPFS_XATTR=y
    CONFIG_HUGETLBFS=n
    CONFIG_MISC_FILESYSTEMS=y
    CONFIG_DEVTMPFS_MOUNT=y

    # AutoFS (required by systemd)
    CONFIG_AUTOFS_FS=y

    # Pseudo filesystems
    CONFIG_PROC_KCORE=n

    # Initial RAM filesystem and RAM disk (initramfs/initrd) support
    CONFIG_BLK_DEV_INITRD=y
    CONFIG_RD_GZIP=y
    CONFIG_RD_BZIP2=y
    CONFIG_RD_LZMA=y
    CONFIG_RD_XZ=y
    CONFIG_RD_LZO=y
    CONFIG_RD_LZ4=y

    # Kernel hacking
    CONFIG_DEBUG_KERNEL=y
    CONFIG_DEBUG_INFO=n
    CONFIG_DEBUG_FS=y
    CONFIG_MAGIC_SYSRQ=y
    CONFIG_PANIC_ON_OOPS=n
    CONFIG_PANIC_TIMEOUT=0
    CONFIG_SCHEDSTATS=n
    CONFIG_DEBUG_BUGVERBOSE=y

    # Security options
    CONFIG_SECURITY=n
    CONFIG_SECURITYFS=n
    CONFIG_SECURITY_NETWORK=n

    # Cryptographic API (required by systemd)
    CONFIG_CRYPTO=y
    CONFIG_CRYPTO_HASH=y
    CONFIG_CRYPTO_HASH2=y
    CONFIG_CRYPTO_CRC32C=y
    CONFIG_CRYPTO_USER_API=y
    CONFIG_CRYPTO_USER_API_HASH=y
    CONFIG_CRYPTO_HMAC=y
    CONFIG_CRYPTO_SHA256=y

    # Library routines
    CONFIG_CRC_CCITT=n
    CONFIG_CRC16=y
    CONFIG_CRC32=y
    CONFIG_CRC32C=y
    CONFIG_ZLIB_INFLATE=y
    CONFIG_ZLIB_DEFLATE=y
    CONFIG_LZO_COMPRESS=y
    CONFIG_LZO_DECOMPRESS=y
    CONFIG_XZ_DEC=y
    CONFIG_ZSTD_COMPRESS=y
    CONFIG_ZSTD_DECOMPRESS=y
  '';

in pkgs.linuxManualConfig {
  inherit (pkgs) stdenv lib;

  version = "6.18.6";
  modDirVersion = "6.18.6";

  src = pkgs.fetchurl {
    url = "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.18.6.tar.xz";
    sha256 = "06x3z649mzwwkb1hvsy0yh7j5jk9qrnwqcmwy7dx8s1ggccrf927";
  };

  configfile = kernelConfig;

  allowImportFromDerivation = true;
}

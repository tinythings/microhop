{ config, pkgs, lib, nixpkgs, microhop, microgen, ... }:

# NixOS system configuration with systemd init and microhop initramfs
# Target: QEMU aarch64 with ext4 rootfs

let
  kernel = import ../kernel.nix { inherit pkgs; };

  microhopConfig = import ./microhop-config.nix { inherit pkgs; };
  
  # Use shared initramfs with microhop
  initramfs = import ../initramfs-microgen.nix {
    inherit pkgs kernel microgen microhop;
    microhopConfig = microhopConfig;
  };

in {
  system.stateVersion = "25.11";

  # Use custom kernel for aarch64
  boot.kernelPackages = pkgs.linuxPackagesFor kernel;

  # Disable GRUB and extlinux, we'll use direct kernel boot
  boot.loader = {
    grub.enable = false;
    generic-extlinux-compatible.enable = false;
  };

  # Use our custom microhop initramfs instead of the standard one
  boot.initrd.enable = true;
  system.build.initialRamdisk = lib.mkForce initramfs;
  
  # Disable hardware detection and modules that may not be available
  hardware.enableRedistributableFirmware = false;
  boot.initrd.availableKernelModules = [];
  boot.initrd.kernelModules = [];
  boot.kernelModules = [];
  boot.extraModulePackages = [];

  # Kernel parameters for aarch64 QEMU
  boot.kernelParams = [
    "console=ttyAMA0,115200"
    "root=/dev/vda"
    "rootfstype=ext4"
    "rw"
    "init=/nix/var/nix/profiles/system/init"
    "systemd.show_status=true"
    "systemd.log_level=info"
  ];

  # ext4 root filesystem
  fileSystems = {
    "/" = {
      device = "/dev/vda";
      fsType = "ext4";
      options = [ "rw" "relatime" ];
    };
  };

  # Systemd configuration for stage 2
  systemd = {
    # Enable systemd as init
    package = pkgs.systemd;
    
    # Additional systemd services
    services = {
      # Custom service to show boot completion
      microhop-boot-complete = {
        description = "Microhop boot completion notification";
        wantedBy = [ "multi-user.target" ];
        after = [ "basic.target" ];
        serviceConfig = {
          Type = "oneshot";
          ExecStart = "${pkgs.coreutils}/bin/echo 'NixOS with microhop and systemd boot complete!'";
          StandardOutput = "journal+console";
        };
      };
    };

    # Enable journal to console for debugging
    settings.Manager = {
      DefaultStandardOutput = "journal+console";
      DefaultStandardError = "journal+console";
    };
  };

  # Network configuration
  networking = {
    hostName = "nixos-systemd";
    useDHCP = false;
    firewall.enable = false;
    
    # Enable networking for QEMU
    interfaces.enp0s1.useDHCP = true;
  };

  # Disable X server
  services.xserver.enable = false;

  # Auto-login as root for testing
  services.getty.autologinUser = "root";

  # User configuration
  users = {
    allowNoPasswordLogin = true;
    mutableUsers = false;
    users.root = {
      hashedPassword = null;
      openssh.authorizedKeys.keys = [];
    };
  };

  # Minimal package set
  environment.systemPackages = with pkgs; [
    microhop
    microgen
    coreutils
    util-linux
    bash
    systemd
  ];

  # Disable unnecessary services for minimal system
  documentation.enable = false;
  documentation.nixos.enable = false;
  security.polkit.enable = false;
  security.rtkit.enable = false;

  # Build outputs for QEMU
  system.build = {
    kernel = lib.mkForce kernel;
    initramfs = lib.mkForce initramfs;
    
    # Use the standard NixOS system closure as rootfs
    rootfs = config.system.build.toplevel;
  };
}
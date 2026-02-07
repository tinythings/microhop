{ config, pkgs, lib, ... }:

# Example NixOS system configuration using microhop
# This is for internal development/testing purposes only
# Not intended for production use

let
  kernel = import ./kernel.nix { inherit pkgs; };
  uboot = import ./u-boot.nix { inherit pkgs; };

  microhop = pkgs.callPackage ../. { };

in {
  system.stateVersion = "25.11";

  boot.kernelPackages = pkgs.linuxPackagesFor kernel;

  boot.loader = {
    grub.enable = false;
    generic-extlinux-compatible.enable = false;
  };

  boot.initrd = {
    enable = true;

    compressor = "gzip";

    extraUtilsCommands = ''
      copy_bin_and_libs ${microhop}/bin/microhop
    '';

    preLVMCommands = lib.mkBefore ''
      echo "Starting microhop minimal init..."
    '';
  };

  environment.systemPackages = with pkgs; [
    microhop
  ];

  boot.kernelParams = [
    "console=ttyAMA0"
    "root=/dev/vda1"
    "init=/init"
  ];

  fileSystems = {
    "/" = {
      device = "/dev/vda1";
      fsType = "ext4";
    };
  };

  networking = {
    hostName = "microhop-test";
    useDHCP = false;
    firewall.enable = false;
  };

  services.xserver.enable = false;

  services.getty.autologinUser = "root";

  users.users.root = { };

  documentation.enable = false;
  documentation.nixos.enable = false;

  environment.noXlibs = true;
  security.polkit.enable = false;
  security.rtkit.enable = false;

  system.build = {
    inherit uboot;

    kernel = config.boot.kernelPackages.kernel;
    initrd = config.system.build.initialRamdisk;
  };
}

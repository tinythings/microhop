{ pkgs ? import <nixpkgs> {} }:

let
  # Use aarch64 packages - let Nix handle the path resolution
  pkgsAarch64 = import pkgs.path {
    system = "aarch64-linux";
  };

  glibcSystemd = pkgsAarch64.systemd;
  glibcNix = pkgsAarch64.nix;
in

pkgs.writeText "microhop.conf" ''
  # Microhop configuration for NixOS with systemd init
  # Kernel modules to load
  modules: []

  # Devices mounting
  # Mount the ext4 rootfs from /dev/vda (whole disk, not partition)
  disks:
    /dev/vda: ext4,/,rw

  # Execute systemd directly from the nix store
  init: ${glibcSystemd}/lib/systemd/systemd

  # Temporary sysroot location
  sysroot: /sysroot

  # Debug log output
  log: debug

  # NixOS configuration
  # Enable NixOS store registration before switch_root
  nixos:
    nix_path: ${glibcNix}
''
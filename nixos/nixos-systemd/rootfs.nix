{ pkgs, nixpkgs }:

let
  # Use regular glibc packages for aarch64 - they'll be downloaded from binary cache
  pkgsAarch64 = import nixpkgs {
    system = "aarch64-linux";
  };

  glibcBash = pkgsAarch64.bash;
  glibcCoreutils = pkgsAarch64.coreutils;
  glibcUtilLinux = pkgsAarch64.util-linux;
  glibcBusybox = pkgsAarch64.busybox;
  glibcSystemd = pkgsAarch64.systemd;
  glibcNix = pkgsAarch64.nix;

  closure = pkgsAarch64.closureInfo {
    rootPaths = [ glibcBash glibcCoreutils glibcUtilLinux glibcBusybox glibcSystemd glibcNix ];
  };

in pkgs.callPackage "${nixpkgs}/nixos/lib/make-ext4-fs.nix" {
  storePaths = [ glibcBash glibcCoreutils glibcUtilLinux glibcBusybox glibcSystemd glibcNix ];
  volumeLabel = "nixos-systemd";
  
  populateImageCommands = ''
    # Create basic directory structure
    mkdir -p ./dev ./proc ./sys ./run ./tmp ./var ./home ./root ./etc ./boot
    mkdir -p ./bin ./sbin ./usr/bin ./usr/sbin ./lib ./lib64
    mkdir -p ./nix/var/nix/profiles ./nix/var/nix/db
    
    chmod 1777 ./tmp
    chmod 755 ./root ./home
    
    # No need for init symlink - microhop config directly references systemd binary
    
    # Create basic system files
    echo "root:x:0:0:root:/root:${glibcBash}/bin/bash" > ./etc/passwd
    echo "root:x:0:" > ./etc/group
    echo "nixos-systemd" > ./etc/hostname
    
    # Create NIXOS tag (required for nixos-rebuild)
    touch ./etc/NIXOS

    # Create fstab
    cat > ./etc/fstab << EOF
/dev/vda / ext4 rw,relatime 0 1
proc /proc proc defaults 0 0
sysfs /sys sysfs defaults 0 0
devtmpfs /dev devtmpfs defaults 0 0
tmpfs /tmp tmpfs defaults 0 0
EOF
  '';
}

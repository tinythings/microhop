{ pkgs ? import <nixpkgs> {} }:

pkgs.buildUBoot {
  version = "2026.01";

  src = pkgs.fetchurl {
    url = "https://ftp.denx.de/pub/u-boot/u-boot-2026.01.tar.bz2";
    sha256 = "sha256-tg1YZc79vHXajaQVbFbEWOAN51pJuAwaLlipbjCtDVQ=";
  };

  defconfig = "qemu_arm64_defconfig";

  extraMeta.platforms = [ "aarch64-linux" ];

  filesToInstall = [ "u-boot.bin" "u-boot" ];

  preBuild = ''
    mkdir -p arch/arm64/include/asm
  '';

  postInstall = ''
    if [ ! -f $out/u-boot.bin ]; then
      echo "Warning: u-boot.bin not found in output"
    fi
  '';
}

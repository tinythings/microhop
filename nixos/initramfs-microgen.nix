{ pkgs, kernel, microgen, microhop, microhopConfig }:

pkgs.stdenv.mkDerivation {
  name = "initramfs-microgen";

  nativeBuildInputs = [ microgen pkgs.cpio pkgs.gzip ];

  unpackPhase = "true";

  buildPhase = ''
    mkdir -p $TMPDIR/rootfs/lib/modules

    if [ -d "${kernel}/lib/modules" ] && [ "$(ls -A ${kernel}/lib/modules)" ]; then
      cp -r ${kernel}/lib/modules/* $TMPDIR/rootfs/lib/modules/
    fi

    mkdir -p $out

    ${microgen}/bin/microgen new \
      --root $TMPDIR/rootfs \
      --config ${microhopConfig} \
      --kernel-config ${kernel.configfile} \
      --filesystems squashfs,ext4 \
      --block-devices virtio_blk,virtio_mmio \
      --output $TMPDIR/build \
      --file $out/initrd
  '';

  installPhase = "true";
}

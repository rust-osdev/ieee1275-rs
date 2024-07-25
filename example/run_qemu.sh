#!/bin/bash
set -e

if [ ! -f disk.img ]; then
fallocate -l 2G disk.img
sfdisk disk.img <<EOF
label: gpt
label-id: 4623E93C-7CA2-3643-9400-6559B3C9F7C6
device: disk.img
unit: sectors
first-lba: 2048
last-lba: 4194270
sector-size: 512

disk.img1 : start=        2048, size=       10240, type=00000000-0000-0000-0000-000000000000, uuid=5E949E3F-BEC6-A84A-BFC0-E67A2D7ACA5F
disk.img2 : start=       12288, size=     4179968, type=0FC63DAF-8483-4772-8E79-3D69D8477DE4, uuid=98371EB3-FB56-0848-A04B-8DF3809ABED0
EOF

fallocate -l 1.5G tmp.img
mkfs.vfat tmp.img
dd if=tmp.img of=disk.img bs=512 seek=12288 conv=notrunc
rm tmp.img

fi

cargo +nightly build --release --target powerpc-unknown-linux-gnu
dd if=target/powerpc-unknown-linux-gnu/release/ieee1275-example of=disk.img bs=512 seek=2048 conv=notrunc
qemu-system-ppc64 -M pseries  -m 512 -bios slof.bin -hda disk.img

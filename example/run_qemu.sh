#!/bin/bash
set -e

if [ ! -f disk.img ]; then
fallocate -l 200M disk.img
sfdisk disk.img <<EOF
label: gpt
label-id: F14A008C-0832-40CC-AB26-B5C4B65BD9D7
device: disk.img
unit: sectors
first-lba: 2048
last-lba: 409566
sector-size: 512

disk.img1 : start=        2048, size=      405504, type=9E1A2D38-C612-4316-AA26-8B49521E5A8B, uuid=C8314A8B-6B60-4ACC-AA68-C743E6076635
EOF

fallocate -l 1.5G tmp.img
mkfs.vfat tmp.img
dd if=tmp.img of=disk.img bs=512 seek=12288 conv=notrunc
rm tmp.img

fi

cargo +nightly build --release --target powerpc-unknown-linux-gnu
dd if=target/powerpc-unknown-linux-gnu/release/ieee1275-example of=disk.img bs=512 seek=2048 conv=notrunc
qemu-system-ppc64 -M pseries  -m 512 -bios slof.bin -hda disk.img

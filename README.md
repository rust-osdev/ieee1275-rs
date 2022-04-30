# OpenFirmware Rust environment

This is an attempt at creating a basic runtime environment much like [uefi-rs](https://github.com/rust-osdev/uefi-rs).

For now this project is only targetting PowerVM/POWER environments as well as QEMU/SLOF. Compatibility with older PowerPC Macs is not a priority, although contributions that are easy to maintain are welcome.


## Build instructions

This is configured as a cross compilation crate by default. The output binaries are required to be PPC32bit big endian. You need a working powerpc gcc compiler, you may need to tweak ```.cargo/config``` to point to the right binaries.

You also need to install the powerpc target toolchain:

```$ rustup target add powerpc-unknown-linux-gnu```

To create a valid target you need to build for release, debug builds will fail due to symbol stripping:

```$ cargo build --relaase --target powerpc-unknown-linux-gnu```

## Testing

You need qemu-system-ppc64le and the SLOF firmware binary, in fedora you can run it by having a disk image with a GPT partition table and a 4MB PReP partition where the binary will be written:

```
$ fallocate -l 2G disk.img
$ cfdisk -z disk.img
                               ┌ Select label type ───┐
                               │◼gpt◼◼◼◼◼◼◼◼◼◼◼◼◼
                               │ dos                  │
                               │ sgi                  │
                               │ sun                  │
                               └──────────────────────┘
                                    Disk: disk.img
                    Size: 2 GiB, 2147483648 bytes, 4194304 sectors
             Label: gpt, identifier: B0FB3AB0-6C25-8D49-97F1-92E9AD879852

    Device               Start          End      Sectors     Size Type
>>  disk.img1             2048        10239         8192       4M PowerPC PReP boot    
    disk.img2            10240      4194270	 4184031       2G Linux filesystem
 ┌───────────────────────────────────────────────────────────────────────────────────┐
 │Partition UUID: 91A7B5D3-6237-834E-A3F2-6D7D87CB3A57                               │
 │Partition type: PowerPC PReP boot (9E1A2D38-C612-4316-AA26-8B49521E5A8B)           │
 └───────────────────────────────────────────────────────────────────────────────────┘
  [ Delete ]  [ Resize ]  [  Quit  ]  [  Type  ]  [  Help  ]  [  Write ]  [  Dump  ]

```

To deploy the image we need to setup the disk image in a loopback device
```
$ sudo losetup -P -f disk.img
$ losetup -a
$ sudo losetup -a
/dev/loop0: [0043]:2771583 (/path/to/disk.img)
$ sudo dd if=target/powerpc-unknown-linux-gnu/release/of-rs of=/dev/loop0p1
```

And finally we need to launch QEMU:
```
$ qemu-system-ppc64 -M pseries-6.1 -bios /usr/share/qemu/slof.bin -drive file=disk.img
```

You should see a "Hello from Rust into Open Firmware" message in the emulator output.
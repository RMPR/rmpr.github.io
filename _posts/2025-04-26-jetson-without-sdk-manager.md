---
layout: post
title: Flashing a Jetson without SDK Manager on Fedora
---

The official instructions to setup Jetson recommend to use [SDK
Manager](https://developer.nvidia.com/sdk-manager) which works fine if you are
using Ubuntu or RHEL/Centos, but not so much if you are using the latest Fedora.

These instructions to flash the SSD of a Jetson Orin have been tested with
Jetson Linux 36.2 (Ubuntu 22.04, Kernel 5.15) but should work with other
subsequent versions of Jetson Linux.

## Put the device in recovery mode

### Jetson Orin Nano

Ensure the device is powered off and the power adapter disconnected. Enable
Force Recovery mode by placing a jumper across the "FC REC" and "GND" pins
located on the edge of the carrier board, under the Jetson Orin Nano module.

### Jetson AGX Orin

Unplug the power cable to the Jetson. Connect the USB cable in the base board Type-C connector (10) to the host machine.
Hold down the force recovery button (the one in the middle), while plugging the power cable back in.
When the power light turns on, release the recovery button.

## Flashing the NVME

The following section has to run bare metal on a host, connected to the jetson with USB-C.

These instructions are derived from
[jetson-linux-flash](https://catalog.ngc.nvidia.com/orgs/nvidia/containers/jetson-linux-flash-x86)

Download the [Driver Package
(BSP)](https://developer.nvidia.com/downloads/embedded/l4t/r36_release_v4.3/release/Jetson_Linux_r36.4.3_aarch64.tbz2)
and the [Sample Root
Filesystem](https://developer.nvidia.com/downloads/embedded/l4t/r36_release_v4.3/release/Tegra_Linux_Sample-Root-Filesystem_r36.4.3_aarch64.tbz2)

Assuming the filename of the L4T Driver Package (BSP) is `${L4T_RELEASE_PACKAGE}`
and the filename of the Sample Root Filesystem is `${SAMPLE_FS_PACKAGE}`, and the
files have been downloaded to `~/workspace`. run the following commands as root
on the host to flash Jetson devices:

```shell
dnf install -y qemu qemu-user-static bmap-tools
export L4T_RELEASE_PACKAGE=Jetson_Linux_r36.4.3_aarch64.tbz2
export SAMPLE_FS_PACKAGE=Tegra_Linux_Sample-Root-Filesystem_r36.4.3_aarch64.tbz2
cd ~/workspace
tar -I lbzip2 -xf ${L4T_RELEASE_PACKAGE}
cd Linux_for_Tegra/rootfs/
tar -I lbzip2 -xpf ../../${SAMPLE_FS_PACKAGE}
cd ..
./tools/l4t_flash_prerequisites.sh
./apply_binaries.sh
./nvsdkmanager_flash.sh --storage nvme0n1p1 # use mmcblk0p1 if you are using an SD card
```

If the host has already run this before, only the following lines are required:

```shell
cd ~/workspace/Linux_for_Tegra
./nvsdkmanager_flash.sh --storage nvme0n1p1
```

And voilà! You can now connect a screen and a keyboard to set up your Jetson.

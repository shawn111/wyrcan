# Wyrcan ~ The Container Bootloader

Wyrcan is a bootloader that **boots** into a container. That's all it does.

But of course, that's not the only thing that Wyrcan *implies*. Using Wyrcan
to boot a container also means that you can use a tried and trusted software
packaging ecosystem to have a bare-metal OS that is:

  * Immutable: All changes to the OS are done by iterating on the container
    pipeline. Consuming those changes means a simple reboot. You can schedule
    reboots to make sure you always have the latest OS.

  * Stateless: Booting a container with Wyrcan means that nothing is installed
    on the disk. There is no state to manage except the state you put into
    your container. You never have to worry about whether packages are updated.
    And if all your mounts of local storage are `noexec`, you can just reboot
    when compromised.

  * Memory-Resident: The full operating system is resident in RAM. That means
    it is fast. However, you can also set up swap in your container so that
    unused pages are written to disk, saving memory for your application.

  * Declarative: Your bare-metal operating system is developed using the same
    delarative tooling that you have come to expect from the container
    development pipeline. But your OS config in git. Host it in your favorite
    git forge (GitHub, GitLab, Bitbucket, etc). Build the images
    automatically. Host them in your favorite container repo.

# Getting Started

There are three basic steps to getting things working:

1. Build and push a *bootable* container.
2. Confingure `wyrcan.img=CONTAINER` to point to the container.
3. Boot Wyrcan.

From there, Wyrcan will do the rest.

## Build and Push a Bootable Container

What is a *bootable* container? A *bootable* container is a container that
includes a kernel (vmlinuz + modules) as well as an init system (usually
`systemd`) properly linked to `/init`.

The steps for this depend on the container's distribution. However, it boils
down to basically three steps.

1. Inhereit from a base image.
2. Install the kernel and the init system.
3. Link to the init system.

After these minimal requirements are met, you can customize the container to
meet your specifications. You will probably build your container with
something like `podman build .` or `docker build .`. Then you can use `podman
push ...` or `docker push ...` to save it in your favorite container repo.

Note that besides the additional requirements outlined above, you use your
*existing* container process.

### Examples
#### Debian

```Dockerfile
FROM debian:latest
RUN apt update
RUN apt install -y linux-image-generic systemd
RUN ln /bin/systemd /init
```

# Configure and Boot Wyrcan

From here we basically want to boot Wyrcan with `wyrcan.img=CONTAINER` in the
kernel `cmdline`. How we accomplish this depends on how are going to boot
Wyrcan. There are two boot methods available: direct boot and ISO boot.

## Direct Boot

The direct boot scheme basically refers to any process that will boot a Linux
`kernel` and `initrd` directly. This includes:

  * PXE / TFTP (i.e. "netboot")
  * iPXE
  * QEMU

The precise details of how to do this depend on the system. However, they all
have the following three options in common: `kernel`, `initrd` (also called
`initramfs`) and `cmdline` (also called `options` or `append`, for historical
reasons).

You basically point the `kernel` and `initrd` options to
[wyrcan.kernel][wyrcan.kernel] and [wyrcan.initrd][wyrcan.initrd],
respectively, and fill in an appropriate `cmdline` value.  We reccomend the
following `cmdline`, however it can be customized to your needs:

```
loglevel=3 systemd.show_status=error wyrcan.img=CONTAINER
```

### Examples
#### QEMU

```sh
$ curl -L 'https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.kernel?job=build' \
  > wyrcan.kernel

$ curl -L 'https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.initrd?job=build' \
  > wyrcan.initrd

$ qemu-system-x86_64 \
  -append "loglevel=3 systemd.show_status=error console=ttyS0 wyrcan.img=CONTAINER" \
  -kernel wyrcan.kernel \
  -initrd wyrcan.initrd \
  -enable-kvm \
  -nographic \
  -m 4G
```

#### iPXE

iPXE only supports a limited number of TLS cipher suites. As of this writing,
iPXE cannot download from GitHub. But it **can** download from GitLab. On
clouds like Equinix Metal, you can specify a URL to the iPXE file you want to
boot and configure the bare metal to always boot that URL. Put the following
file in a GitLab repo and put the raw URL to it in your cloud provider. Now
your boot is completely automated.

```ipxe
#!ipxe

set kernel https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.kernel?job=build
set initrd https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.initrd?job=build

kernel ${kernel} loglevel=3 systemd.show_status=error console=ttyS0 wyrcan.img=CONTAINER
initrd ${initrd}
boot
```

## ISO Boot

The ISO boot scheme differs from the direct boot scheme in that since we are
booting Wyrcan from an ISO (either as an image or as burned onto a CD, DVD or
USB storage device), we cannot specify the `cmdline` directly.  Therefore we
can either specify it manually or by persisting it into an EFI variable.

### Manual

This method is the most direct. However, it lacks automation. It is, however,
useful for exploration and testing.

1. Download the [Wyrcan ISO][wyrcan.iso].
2. Burn it onto a CD, DVD or USB storage device (optional).
3. Boot it.
4. When you get to the bootloader menu, press `e`. This brings up the editor
   for the `cmdline`.
5. Edit the `cmdline` to meet your needs. Make sure to specify
   `wyrcan.img=CONTAINER`.
6. Press `ENTER` or `RETURN` to boot.

From here you should see Wyrcan download the specified container and boot it.

While this works great, we need a way to automate the boot process.

### Automated

In order to automate the ISO boot process, we need a way to persist the
`cmdline`. In order to do this, just add `wyrcan.efi=write` to the `cmdline`.
Wyrcan will validate your configuration and then store your `cmdline` in an
EFI variable. Once this is complete, Wyrcan will immediately reboot to show
you the fully automated process.

Once the `cmdline` is written to an EFI variable, you no longer need to edit
the `cmdline` manually. Just let Wyrcan boot from the default option. Wyrcan
will find your previously saved `cmdline` and boot it. Just leave the ISO
connected to the device and Wyrcan will do the right thing every time you
reboot.

#### ⚠⚠⚠ WARNING ⚠⚠⚠

Some older systems have EFI firmware bugs that can cause the hardware to be
bricked if you write a custom EFI variable. This is a clear violation of the
EFI specification, but is rather unfortunate. It is unlikely that you have
such hardware, particularly if you are using Wyrcan on a server platform.
However, you should be aware that there is some minor risk that using the EFI
variable automation flow could brick your system.

### Examples

#### QEMU (EFI-only)

This is slightly more complex only because you have to be sure that qemu is
using EFI. Wyrcan does not support BIOS-only systems.

1. Download the [Wyrcan ISO][wyrcan.iso] image.

2. Duplicate `OVMF_VARS.fd`. This is what gives you the ability to save EFI
   variables. This file should be included with your distribution if you have
   `qemu` installed. For details, see your distribution's documentation.

3. Boot the ISO using EFI (be sure to allocate enough RAM). Like above, the
   `OVMF_CODE.fd` file should be included with your distribution.

```sh
$ curl -L 'https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.iso?job=build' \
  > wyrcan.iso

$ cp /usr/share/edk2/ovmf/OVMF_VARS.fd myvars.fd

$ qemu-system-x86_64 \
  -drive if=pflash,format=raw,readonly,file=/usr/share/edk2/ovmf/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=myvars.fd \
  -cdrom wyrcan.iso \
  -enable-kvm \
  -nographic \
  -m 4G
```

# Frequently Asked Questions (FAQ)

## How does Wyrcan work?

Wyrcan boots a tiny build of Linux. Once network is up, Wyrcan downloads the
specified container image. Next, Wyrcan converts the container image to a
`kernel` and `initrd`. The `initrd` includes the entire container image
besides the kernel. Finally, Wyrcan boots the resulting `kernel`/`initrd`
using the `kexec` facility.

Yes, the *actual* kernel from the container image is booted. Once the
container has booted, nothing from Wyrcan stays resident in memory.

## Wait... Memory-Resident... Are you using all my RAM!?

Yes. But not really. Modern operating systems always try to use all your RAM.
It is simply more efficient.

Applications and data that are disk-resident and are frequently used are
cached in memory. Memory pages that aren't frequently used get swapped to
disk. In fact, on any modern `systemd`-based Linux OS there are a variety of
tmpfs mounts all over the system. All of this is quite normal.

It is true that the booted container is entirely memory resident. But it is
also true that its unused pages will get swapped to disk if you set up a swap
partition or swap file on the disk inside your container. So under either
system, the used data stays in RAM and the unused data ends up in the swap on
disk.

Also... Have you seen servers these days? You can get 2TiB of memory in
bare-metal cloud servers. You'll be fine.

## Why is Wyrcan hosted on GitLab?

Unfortunately, iPXE has limited support for TLS cipher suites. The consequence
of this is that iPXE can download the Wyrcan `kernel` and `initrd` files
directly from GitLab but not from GitHub. I haven't tried other git forges.

# Download Links

* [wyrcan.kernel][wyrcan.kernel]
* [wyrcan.initrd][wyrcan.initrd]
* [wyrcan.iso][wyrcan.iso]

[wyrcan.kernel]: https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.kernel?job=build
[wyrcan.initrd]: https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.initrd?job=build
[wyrcan.iso]: https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/main/raw/wyrcan.iso?job=build

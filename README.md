# Wyrcan ~ The Container Bootloader

Wyrcan is a bootloader that **boots** into a container. That's all it does.

But of course, that's not the only thing that Wyrcan *implies*. Using Wyrcan
to boot a container also means that you can use a tried and trusted software
packaging ecosystem to have a bare-metal OS that is:

  * Stateless: Booting a container with Wyrcan means that nothing is installed
    on the disk. There is no state to manage except the state you put into
    your container. You never have to worry about whether packages are updated:
    you can schedule reboots to make sure you always have the latest OS. And if
    all your mounts of local storage are `noexec`, you can just reboot when
    compromised.

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
2. Configure `wyrcan.img=CONTAINER` to point to the container.
3. Boot Wyrcan.

From there, Wyrcan will do the rest.

## Build and Push a Bootable Container

What is a *bootable* container? A *bootable* container is a normal container
with a few additional customizations:

1. A kernel is installed and symlinked to `/boot/wyrcan.kernel`.

2. An init system (usually systemd) is installed and symlinked to `/init`.

3. (Optional) All necessary hardware support packages are installed. This may
   include firmware, userspace drivers or enabling software. Often nothing is
   expressly required for this step as installing the kernel will pull in all
   the necessary dependencies.

4. (Optional) The container may defined required kernel cmdline options by
   specifying them in `/boot/wyrcan.cmdline`.

After these minimal requirements are met, you can customize the container to
meet your specifications. You will probably build your container with
something like `podman build .` or `docker build .`. Then you can use `podman
push ...` or `docker push ...` to save it in your favorite container repo.

Note that besides the additional requirements outlined above, you use your
*existing* container process.

### Examples

The Wyrcan project provides a number of *bootable* containers for mainstream
Linux distributions. All of these are available in the GitLab Container
Registry. New features and distributions, as well as bug fixes, are welcome!

1. Debian

  * Container Image: `registry.gitlab.com/wyrcan/debian`
  * Dockerfile: https://gitlab.com/wyrcan/debian

2. Fedora

  * Container Image: `registry.gitlab.com/wyrcan/fedora`
  * Dockerfile: https://gitlab.com/wyrcan/fedora

3. Ubuntu

  * Container Image: `registry.gitlab.com/wyrcan/ubuntu`
  * Dockerfile: https://gitlab.com/wyrcan/ubuntu

4. Arch Linux

  * Container Image: `registry.gitlab.com/wyrcan/archlinux`
  * Dockerfile: https://gitlab.com/wyrcan/archlinux

5. CentOS

  * Container Image: `registry.gitlab.com/wyrcan/centos`
  * Dockerfile: https://gitlab.com/wyrcan/centos

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
quiet wyrcan.arg=loglevel=3 wyrcan.img=CONTAINER
```

### Examples
#### QEMU

```sh
$ curl -L 'https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.kernel?job=images' \
  > wyrcan.kernel

$ curl -L 'https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.initrd?job=images' \
  > wyrcan.initrd

$ qemu-system-x86_64 \
  -append "console=ttyS0 quiet wyrcan.arg=loglevel=3 wyrcan.img=CONTAINER" \
  -kernel wyrcan.kernel \
  -initrd wyrcan.initrd \
  -enable-kvm \
  -nographic \
  -m 4G
```

[![asciicast](https://asciinema.org/a/450003.svg)](https://asciinema.org/a/450003?autoplay=1)

#### iPXE

iPXE only supports a limited number of TLS cipher suites. As of this writing,
iPXE cannot download from GitHub. But it **can** download from GitLab. On
clouds like Equinix Metal, you can specify a URL to the iPXE file you want to
boot and configure the bare metal to always boot that URL. Put the following
file in a GitLab repo and put the raw URL to it in your cloud provider. Now
your boot is completely automated.

```ipxe
#!ipxe

set kernel https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.kernel?job=images
set initrd https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.initrd?job=images

kernel ${kernel} console=ttyS0 quiet wyrcan.arg=loglevel=3 wyrcan.img=CONTAINER
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

In order to automate the ISO boot process, we need a way to persist
`wyrcan.img` and `wyrcan.arg`. In order to do this, just add `wyrcan.efi=write`
to the `cmdline`. Wyrcan will validate your configuration and store it into
EFI NVRAM. Once this is complete, Wyrcan will reboot to show you the fully
automated process.

Once once the arguments are stored, you no longer need to edit the `cmdline`
manually. Just let Wyrcan boot from the default option. Wyrcan will find your
previously saved `cmdline` and boot it. Just leave the ISO connected to the
device and Wyrcan will do the right thing every time you reboot.

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
$ curl -L 'https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.iso?job=images' \
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

[![asciicast](https://asciinema.org/a/450009.svg)](https://asciinema.org/a/450009?autoplay=1)

# Frequently Asked Questions (FAQ)

## How does Wyrcan work?

Wyrcan boots a tiny build of Linux. Once network is up, Wyrcan downloads the
specified container image. Next, Wyrcan converts the container image to a
`kernel` and `initrd`. The `initrd` includes the entire container image
besides the kernel. Finally, Wyrcan boots the resulting `kernel`/`initrd`
using the `kexec` facility.

Yes, the *actual* kernel from the container image is booted. Once the
container has booted, nothing from Wyrcan stays resident in memory.

## Can Wyrcan boot itself?

Yes! Wyrcan is itself a bootable container. This means you can chain load
Wyrcan to ensure the latest version. For example, the following cmdline
would first boot into the latest release of Wyrcan before booting the final
image:

```
wyrcan.img=registry.gitlab.com/wyrcan/wyrcan wyrcan.arg=wyrcan.img=registry.gitlab.com/wyrcan/debian
```

You can see a working chained boot here:

[![asciicast](https://asciinema.org/a/450016.svg)](https://asciinema.org/a/450016?autoplay=1)

## What is the full list of Wyrcan kernel command line arguments?

You can use the following kernel cmdline arguments to control Wyrcan:

  * `wyrcan.img=IMG` - Specifies which container will be booted. IMG should be
    a container name in the usual format. For example:

    ```
    wyrcan.img=registry.gitlab.com/wyrcan/debian:latest
    ```

  * `wyrcan.arg=ARG` - Passes the specified cmdline arguments to the container's
    kernel. This argument may be specified multiple times and may be quoted to
    include spaces. The arguments passed within will be ignored by the Wyrcan
    kernel. For example, the following is valid:

    ```
    wyrcan.arg="quiet log-buf-len=1M" wyrcan.arg=print-fatal-signals=1
    ```

    The container's kernel will receive the following cmdline:

    ```
    quiet log-buf-len=1M print-fatal-signals=1
    ```

  * `wyrcan.efi=write` - Saves the wyrcan.img and wyrcan.arg parameters to EFI
    NVRAM. This enables persistent, automated boot.

  * `wyrcan.efi=clear` - Removes all previously stored values from EFI NVRAM.
    This disables persistent, automated boot.

## Wait... Memory-Resident... Are you using all my RAM!?

Yes. But not really. Modern operating systems always try to use all your RAM.
It is simply more efficient.

In a traditional boot, the OS starts out on disk. As the OS and its
applications are loaded, the data is transferred from disk to memory pages.
When those applications exit, the pages often stay cached in memory. Those
pages are often written back to disk in a swap file or partition.

In Wycan, the OS starts out in memory. It is true that this requires more
memory to boot than a traditional system. However, if you boot a container
that enables a swap file or partition on disk, the unused portion of the OS
will be paged out to disk just like a normal OS. So the end result is somewhat
similar. However, with Wyrcan, you don't have to manage the state of an OS on
disk. And that is a big win.

Also... Have you seen servers these days? You can get 2TiB of memory in
bare-metal cloud servers. You'll be fine.

## Why is Wyrcan hosted on GitLab?

Unfortunately, iPXE has limited support for TLS cipher suites. The consequence
of this is that iPXE can download the Wyrcan `kernel` and `initrd` files
directly from GitLab. Other git forges I tried didn't work.

Also, GitLab is open source and has a built-in container registry. There's a
lot to like!

# Download Links

* [wyrcan.kernel][wyrcan.kernel]
* [wyrcan.initrd][wyrcan.initrd]
* [wyrcan.iso][wyrcan.iso]

[wyrcan.kernel]: https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.kernel?job=images
[wyrcan.initrd]: https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.initrd?job=images
[wyrcan.iso]: https://gitlab.com/wyrcan/wyrcan/-/jobs/artifacts/latest/raw/wyrcan.iso?job=images

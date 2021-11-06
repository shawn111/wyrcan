# Wyrcan ~ Memory-Resident, Container Bootloader

Wyrcan is an immutable, memory-resident, container bootloader. Think of it
something like iPXE, except that the only configuration is the name of the
container you want to boot.

Containers, at heart, are a packaging system. They bundle up just enough OS to
run an application. Why not boot an OS in the same way?

And, yes, I said "boot." No, Wycan doesn't launch a container underneath
another OS. It actually **boots** the contents of a container.

# Why is Wyrcan Useful?

Modern application development basically follows this workflow:

1. Build an application using git.
2. Push your branch to GitHub, GitLab, etc.
3. CI/CD assembles a container of your application to deploy.
4. Schedule the application to be deployed with k8s.

But bare metal servers still live in the dark ages. Typically, you have to:

1. Get an ISO.
2. Somehow attach it to a physical server (or a cloud).
3. Install the OS.
4. Manage the OS (often using tools such as Chef, Puppet or Ansible).

Wouldn't it be great if we could just have declarative, immutable
infrastructure for bare metal too? Now you can!

# How Does Wyrcan Work?

## Booting Wyrcan

The first step is to get Wyrcan up and running on your hardware. This should
be easy. All the files you need to do this are available directly from GitLab.
There are two options;

1. Download the Wyrcan ISO and burn it onto a CD or copy it onto a USB storage
   device. Your physical server can now boot from this device. Upon booting for
   the first time, you will be given a boot menu. You can edit the kernel's
   cmdline by pressing `e`. Add any boot parameters you need as well as (where
   `CONTAINER` is the container image you want to boot):

   ```
   wyrcan.img=CONTAINER
   ```

   This will boot the specified container image. If you encounter problems, just
   reboot! No permanent changes were made to your system. If you want to persist
   the configuration on this system, just add these to the `cmdline`:

   ```
   wyrcan.img=CONTAINER wyrcan.efi=write
   ```

   This will save your `cmdline` in an EFI variable. From now on, when you boot
   the Wyrcan ISO, it will use your `cmdline` automatically. So long as you
   boot from the Wyrcan ISO, the boot process will be fully automated.

2. Load Wyrcan using a network boot. You can download the `kernel` and
   `initrd` from GitLab and put them on a TFTP server for PXE booting. Or,
   alternatively, you can use iPXE and download the `kernel` and `initrd`
   directly from GitLab during boot.

   With this method, you can just set the kernel `cmdline` directly. No need
   to persist anything to EFI. Just set the `wyrcan.img=...` value along with
   anything else you need for boot to work.

## But what does Wyrcan Do?

Wyrcan boots a tiny build of Linux. Once network is up, Wyrcan downloads the
specified container image. Next, Wyrcan converts the container image to a
`kernel` and `initrd`. The `initrd` includes the entire container image
besides the kernel. Finally, Wyrcan boots the resulting `kernel`/`initrd`
using the `kexec` facility.

Yes, the **actual** kernel from the container image is booted. Once the
container has booted, nothing from Wyrcan stays resident in memory.

## Wait... Memory-resident... Are you using all my RAM!?

Yes. But not really. Modern operating systems always try to use all your RAM.
It is simply more efficient.

Applications and data that are disk-resident and are frequently used are
cached in memory. Memory pages that aren't frequently used get swapped to
disk. In fact, on any modern `systemd`-based Linux OS there are a variety of
tmpfs mounts all over the system. All of this is quite normal.

It is true that the booted container is entirely memory resident. But it is
also true that its unused pages will get swapped to disk if you set up a swap
partition or file on the disk inside your container. So under either system,
the used data stays in RAM and the unused data ends up in the swap on disk.

Also... Have you seen servers these days? You can get 2TiB of memory in
bare-metal cloud servers. You'll be fine.

## So, what does all this mean?

It means you can build a fully-automated, highly-scalable, bare-metal
infrastructure using just your favorite git forge and your favorite bare-metal
hosting provide.

1. Build your OSes as containers. Check in the Dockerfiles. Have the images
   build automatically in the standard CI/CD pipeline workflows.

2. Put any ancillary infrastructure files, like iPXE scripts, in git too.

3. Point your bare-metal instances at Wyrcan using iPXE or a standard ISO
   file. Point your Wyrcan instances at your containers.

4. Way to go! You have a fully automated, declarative infrastructure with no
   managment servers in the middle. And it is all just containers.

## Can I try Wyrcan in QEMU?

Yes! In fact, we test with QEMU. So this should all work pretty well. You can
choose how you want to test Wyrcan. You can either load the `kernel` and
`initrd` directly or you can boot the ISO image.

### Direct Boot

1. Download the `kernel` and `initrd` image.

2. Run them in QEMU directly (be sure to allocate enough RAM).

```sh
$ qemu-system-x86_64 \
  -append "loglevel=3 systemd.show_status=error console=ttyS0 wyrcan.img=CONTAINER" \
  -kernel ./kernel \
  -initrd ./initrd \
  -enable-kvm \
  -nographic \
  -m 4G
```

### ISO Boot

This method is slightly more complex only because you have to be sure that qemu is using EFI.

1. Download the `wyrcan.iso` image.

2. Duplicate `OVMF_VARS.fd`. This is what gives you the ability to save EFI
   variables. This file should be included with your distribution if you have
   `qemu` installed. For details, see your distribution's documentation.

3. Boot the ISO using EFI (be sure to allocate enough RAM). Like above, the
   `OVMF_CODE.fd` file should be included with your distribution.

```sh
$ cp /usr/share/edk2/ovmf/OVMF_VARS.fd myvars.fd
$ qemu-system-x86_64 \
  -drive if=pflash,format=raw,readonly,file=/usr/share/edk2/ovmf/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=./myvars.fd \
  -cdrom ./wyrcan.iso \
  -enable-kvm \
  -nographic \
  -m 4G
```

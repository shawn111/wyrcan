#!/bin/bash -e

KERNEL=`realpath $1`
INITRD=`realpath $2`
OUTPUT=`realpath $3`

ISO=`mktemp -d` # ISO files go here
TMP=`mktemp -d` # Other files go here

function cleanup() {
  rm -rf $ISO $TMP
}
trap cleanup EXIT

EFI=usr/lib/systemd/boot/efi
LEN=`stat -c "%s" "$INITRD"`
IMG=$TMP/efi.img

# Create the FAT32 image (10% larger than the initrd)
truncate -s $(($LEN * 11 / 10)) $IMG
mkfs.vfat -n 'WYRCAN' $IMG

# Extract the bootloader from the $INITRD and add it to the $IMG
(cd $TMP; bsdtar -x -f "$INITRD" $EFI/systemd-boot*.efi)
arch=`ls $TMP/$EFI/ | sed -rn 's|systemd-boot([^.]+).efi|\1|p'`
mmd -i $IMG ::EFI
mmd -i $IMG ::EFI/BOOT
mcopy -i $IMG $TMP/$EFI/systemd-boot${arch}.efi ::EFI/BOOT/boot${arch}.efi

# Create the bootloader config and add it to the $IMG
echo "timeout  5" >> $TMP/loader.conf
echo "editor yes" >> $TMP/loader.conf
mmd -i $IMG ::loader
mcopy -i $IMG $TMP/loader.conf ::loader/

# Create the bootloader entry and add it to the $IMG
echo "title Wyrcan ~ The Container Bootloader" >> $TMP/wyrcan.conf
echo "linux  /kernel" >> $TMP/wyrcan.conf
echo "initrd /initrd" >> $TMP/wyrcan.conf
echo "options quiet wyrcan.skip=quiet" >> $TMP/wyrcan.conf
mmd -i $IMG ::loader/entries
mcopy -i $IMG $TMP/wyrcan.conf ::loader/entries/

# Add the kernel and initrd to the $IMG
mcopy -i $IMG "$KERNEL" ::kernel
mcopy -i $IMG "$INITRD" ::initrd

# Generate the ISO
xorriso -as mkisofs -o "$OUTPUT" \
    -append_partition 2 0xef $IMG \
    -R -J -joliet-long \
    -iso-level 3 \
    -V WYRCAN \
    $ISO

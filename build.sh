#!/bin/bash -e

ZIP="xz -9 -C crc32"
[ "$1" == "quick" ] && LAYERS="--layers"
[ "$1" == "quick" ] && ZIP="cat"
[ "$1" != "build" ] && UNSHARE="buildah unshare"

if [ "$1" == "clean" ]; then
  rm -f kernel initrd wyrcan.iso
  exit 0
fi

# Generate some UUIDs to avoid collisions.
IMAGE=`cat /proc/sys/kernel/random/uuid`

# Build and mount the container image.
buildah bud $LAYERS -t "$IMAGE" container/

# Clean up the image when complete.
function cleanup() {
  buildah rmi "$IMAGE"
}
[ "$1" != "quick" ] && trap cleanup EXIT

# Extract the kernel and initrd.
$UNSHARE container/wyrcan-extract \
    -k kernel \
    -i initrd \
    -z "$ZIP" \
    "$IMAGE"

# Build the ISO
./iso.sh kernel initrd wyrcan.iso

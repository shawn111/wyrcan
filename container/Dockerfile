FROM debian:latest

RUN apt-get update && \
    apt-get install --no-install-recommends -y linux-image-generic && \
    apt-get install --no-install-recommends -y ca-certificates && \
    apt-get install --no-install-recommends -y kexec-tools && \
    apt-get install --no-install-recommends -y buildah && \
    apt-get install --no-install-recommends -y systemd && \
    rm -rf /var/lib/apt/lists/*

RUN cd /boot; ln vmlinuz-* wyrcan.kernel
RUN ln /bin/systemd /init

# Enable networking.
# Try to autoconfig all interfaces.
# Network is online when ANY iface is online.
COPY systemd-networkd-wait-online.service.override /etc/systemd/system/systemd-networkd-wait-online.service.d/override.conf
COPY autoconf.network /etc/systemd/network/autoconf.network
RUN systemctl enable systemd-resolved.service systemd-networkd.service

# Disable all login services
RUN systemctl mask systemd-logind.service serial-getty@.service getty@.service

# Set up the wyrcan boot service
COPY wyrcan.service /etc/systemd/system/wyrcan.service
COPY wyrcan-extract /usr/local/bin/wyrcan-extract
COPY wyrcan-boot /usr/local/bin/wyrcan-boot
RUN systemctl enable wyrcan.service

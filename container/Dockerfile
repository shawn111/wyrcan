FROM debian:latest

RUN apt-get update && \
    apt-get install --no-install-recommends -y linux-image-generic && \
    apt-get install --no-install-recommends -y ca-certificates && \
    apt-get install --no-install-recommends -y systemd && \
    rm -rf /var/lib/apt/lists/*

RUN cd /boot; ln -s vmlinuz-* wyrcan.kernel
RUN ln -s /bin/systemd /init

# Enable networking.
COPY autoconf.override /etc/systemd/system/systemd-networkd-wait-online.service.d/override.conf
COPY autoconf.network /etc/systemd/network/autoconf.network
RUN systemctl enable systemd-resolved.service systemd-networkd.service

# Disable all login services
RUN systemctl mask systemd-logind.service console-getty.service serial-getty@.service getty@.service getty.target

# Set up the wyrcan boot service
COPY wyrcan.service /etc/systemd/system/wyrcan.service
COPY wyrcan /usr/local/bin/wyrcan
RUN systemctl enable wyrcan.service

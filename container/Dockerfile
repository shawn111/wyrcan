FROM registry.gitlab.com/wyrcan/debian:bookworm

# Install packages
RUN sed -i 's| main| main contrib non-free|' /etc/apt/sources.list
RUN apt update \
 && apt install -y ca-certificates zram-tools firmware-linux firmware-realtek default-dbus-session-bus \
 && rm -rf /var/lib/apt/lists/*

# Enable zram.
COPY zramswap /etc/default/zramswap

# Enable networking.
COPY wait-online.override /etc/systemd/system/systemd-networkd-wait-online.service.d/override.conf
RUN systemctl enable systemd-resolved.service systemd-networkd.service

# Disable all login services
RUN systemctl mask console-getty.service serial-getty@.service getty@.service getty.target

# Set up the wyrcan
COPY rescue.service /etc/systemd/system/
COPY wyrcan.service /etc/systemd/system/
COPY wyrnet.service /etc/systemd/system/
COPY wyrefi.service /etc/systemd/system/
COPY wyrcan /usr/local/bin/
RUN systemctl enable wyrcan.service wyrnet.service wyrefi.service
